use crate::config::Config;
use crate::control_flow_graph::{CFGCollector, ControlFlowGraph};
use crate::predicate_analysis::PredicateAnalyzer;
use crate::predicates::{Predicate, SerializedPredicate};
use crate::trace::{Instruction, Selector, Trace, TraceVec};
use crate::trace_integrity::TraceIntegrityChecker;
use glob::glob;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::{read_to_string, File};
use std::io::Write;
use std::process::exit;

pub struct TraceAnalyzer {
    pub crashes: TraceVec,
    pub non_crashes: TraceVec,
    pub address_scores: HashMap<usize, Predicate>,
    pub cfg: ControlFlowGraph,
    pub memory_addresses: MemoryAddresses,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MemoryAddresses {
    pub heap_start: Option<usize>,
    pub heap_end: Option<usize>,
    pub stack_start: Option<usize>,
    pub stack_end: Option<usize>,
}

impl MemoryAddresses {
    pub fn read_from_file(config: &Config) -> MemoryAddresses {
        let file_path = format!("{}/addresses.json", config.output_directory);
        let content =
            fs::read_to_string(&file_path).expect(&format!("File {} not found!", &file_path));
        serde_json::from_str(&content).expect(&format!("Could not deserialize file {}", &file_path))
    }
}

fn store_trace(trace: &Trace, must_have: &Option<HashSet<usize>>) -> bool {
    match must_have {
        Some(addresses) => addresses.iter().any(|k| trace.instructions.contains_key(k)),
        None => true,
    }
}

pub fn read_crash_blacklist(
    blacklist_crashes: bool,
    crash_blacklist_path: &String,
) -> Option<Vec<String>> {
    if blacklist_crashes {
        Some(
            read_to_string(crash_blacklist_path)
                .expect("Could not read crash blacklist")
                .split("\n")
                .map(|s| {
                    s.split("/")
                        .last()
                        .expect(&format!("Could not split string {}", s))
                        .to_string()
                })
                .filter(|s| !s.is_empty())
                .collect(),
        )
    } else {
        None
    }
}

pub fn blacklist_path(path: &String, blacklist: &Option<Vec<String>>) -> bool {
    blacklist
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .any(|p| path.contains(p))
}

fn parse_traces(
    path: &String,
    config: &Config,
    must_include: Option<HashSet<usize>>,
    blacklist_paths: Option<Vec<String>>,
) -> TraceVec {
    let pattern = match config.zipped {
        false => format!("{}/*trace", path),
        true => format!("{}/*.zip", path),
    };

    let mut paths: Vec<String> = glob(&pattern)
        .unwrap()
        .map(|p| p.unwrap().to_str().unwrap().to_string())
        .filter(|p| !blacklist_path(&p, &blacklist_paths))
        .collect();

    if config.random_traces() {
        paths.shuffle(&mut thread_rng());
    }

    match config.zipped {
        false => TraceVec::from_vec(
            paths
                .into_par_iter()
                .map(|s| Trace::from_trace_file(s))
                .take(if config.random_traces() {
                    config.random_traces
                } else {
                    0xffff_ffff_ffff_ffff
                })
                .filter(|t| store_trace(&t, &must_include))
                .collect(),
        ),
        true => TraceVec::from_vec(
            paths
                .into_par_iter()
                .map(|s| Trace::from_zip_file(s))
                .take(if config.random_traces() {
                    config.random_traces
                } else {
                    0xffff_ffff_ffff_ffff
                })
                .filter(|t| store_trace(&t, &must_include))
                .collect(),
        ),
    }
}

impl TraceAnalyzer {
    pub fn new(config: &Config) -> TraceAnalyzer {
        println!("reading crashes");
        let crash_blacklist =
            read_crash_blacklist(config.blacklist_crashes(), &config.crash_blacklist_path);
        let crashes = parse_traces(&config.path_to_crashes, config, None, crash_blacklist);
        let crashing_addresses: Option<HashSet<usize>> = match config.filter_non_crashes {
            true => Some(crashes.iter().map(|t| t.last_address).collect()),
            false => None,
        };

        println!("reading non-crashes");
        let non_crashes = parse_traces(
            &config.path_to_non_crashes,
            config,
            crashing_addresses,
            None,
        );

        println!(
            "{} crashes and {} non-crashes",
            crashes.len(),
            non_crashes.len()
        );

        let mut trace_analyzer = TraceAnalyzer {
            crashes,
            non_crashes,
            address_scores: HashMap::new(),
            cfg: ControlFlowGraph::new(),
            memory_addresses: MemoryAddresses::read_from_file(config),
        };

        if config.check_traces || config.dump_scores || config.debug_predicate() {
            let mut cfg_collector = CFGCollector::new();
            println!("filling cfg");
            trace_analyzer.fill_cfg(&mut cfg_collector);
        }

        if config.check_traces {
            println!("checking traces");
            TraceIntegrityChecker::check_traces(&trace_analyzer);
            exit(0);
        }

        if config.dump_scores {
            println!("calculating scores");
            trace_analyzer.fill_address_scores();
        }

        trace_analyzer
    }

    fn fill_cfg(&mut self, cfg_collector: &mut CFGCollector) {
        for instruction in self
            .crashes
            .iter_all_instructions()
            .chain(self.non_crashes.iter_all_instructions())
        {
            for succ in &instruction.successors {
                cfg_collector.add_edge(instruction.address, succ.address);
            }
        }

        self.cfg = cfg_collector.construct_graph();
    }

    fn fill_address_scores(&mut self) {
        let addresses = self.crash_non_crash_intersection();
        self.address_scores = addresses
            .into_par_iter()
            .map(|address| {
                (
                    address,
                    PredicateAnalyzer::evaluate_best_predicate_at_address(address, self),
                )
            })
            .collect();
    }

    pub fn address_union(&self) -> HashSet<usize> {
        let crash_union = TraceAnalyzer::trace_union(&self.crashes);
        let non_crash_union = TraceAnalyzer::trace_union(&self.non_crashes);
        crash_union.union(&non_crash_union).map(|x| *x).collect()
    }

    pub fn crash_address_union(&self) -> HashSet<usize> {
        TraceAnalyzer::trace_union(&self.crashes)
    }

    fn trace_union(traces: &TraceVec) -> HashSet<usize> {
        let mut res = HashSet::new();
        for trace in traces.iter() {
            res = res.union(&trace.visited_addresses()).map(|x| *x).collect();
        }

        res
    }

    pub fn iter_all_instructions<'a>(
        crashes: &'a TraceVec,
        non_crashes: &'a TraceVec,
    ) -> impl Iterator<Item = &'a Instruction> {
        crashes
            .iter_all_instructions()
            .chain(non_crashes.iter_all_instructions())
    }

    pub fn iter_all_traces(&self) -> impl Iterator<Item = &Trace> {
        self.crashes.iter().chain(self.non_crashes.iter())
    }

    pub fn iter_all_instructions_at_address(
        &self,
        address: usize,
    ) -> impl Iterator<Item = &Instruction> {
        self.crashes
            .iter_instructions_at_address(address)
            .chain(self.non_crashes.iter_instructions_at_address(address))
    }

    pub fn crash_non_crash_intersection(&self) -> HashSet<usize> {
        let crash_union = TraceAnalyzer::trace_union(&self.crashes);
        let non_crash_union = TraceAnalyzer::trace_union(&self.non_crashes);
        crash_union
            .intersection(&non_crash_union)
            .map(|x| *x)
            .collect()
    }

    pub fn values_at_address(
        &self,
        address: usize,
        selector: &Selector,
        reg_index: Option<usize>,
    ) -> Vec<u64> {
        let ret: Vec<_> = match selector {
            Selector::RegMin => self
                .iter_all_instructions_at_address(address)
                .filter(|i| i.registers_min.get(reg_index.unwrap()).is_some())
                .map(|i| i.registers_min.get(reg_index.unwrap()).unwrap().value())
                .collect(),
            Selector::RegMax => self
                .iter_all_instructions_at_address(address)
                .filter(|i| i.registers_max.get(reg_index.unwrap()).is_some())
                .map(|i| i.registers_max.get(reg_index.unwrap()).unwrap().value())
                .collect(),
            _ => unreachable!(),
        };

        ret
    }

    pub fn unique_values_at_address(
        &self,
        address: usize,
        selector: &Selector,
        reg_index: Option<usize>,
    ) -> Vec<u64> {
        let mut ret: Vec<_> = self
            .values_at_address(address, selector, reg_index)
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        ret.sort();

        ret
    }

    pub fn sort_scores(&self) -> Vec<Predicate> {
        let mut ret: Vec<Predicate> = self
            .address_scores
            .iter()
            .map(|(_, p)| (p.clone()))
            .collect();

        ret.par_sort_by(|p1, p2| p1.score.partial_cmp(&p2.score).unwrap());

        ret
    }

    pub fn dump_scores(&self, config: &Config, filter_scores: bool, print_scores: bool) {
        let (file_name, scores) = (
            format!("{}/scores_linear.csv", config.output_directory),
            self.sort_scores(),
        );

        let mut file = File::create(file_name).unwrap();

        for predicate in scores.iter() {
            if filter_scores && predicate.score <= 0.5 {
                continue;
            }

            write!(
                &mut file,
                "{:#x};{} ({}) -- {}\n",
                predicate.address,
                predicate.score,
                predicate.to_string(),
                self.get_any_mnemonic(predicate.address),
            )
            .unwrap();
        }

        if print_scores {
            TraceAnalyzer::print_scores(&scores, filter_scores);
        }

        TraceAnalyzer::dump_for_serialization(config, &scores)
    }

    fn dump_for_serialization(config: &Config, scores: &Vec<Predicate>) {
        let scores: Vec<_> = scores.iter().map(|p| p.to_serialzed()).collect();
        let serialized_string = serde_json::to_string(&scores).unwrap();

        let file_path = format!("{}/scores_linear_serialized.json", config.output_directory);

        fs::write(&file_path, serialized_string)
            .expect(&format!("Could not write file {}", file_path));
    }

    pub fn get_predicates_better_than(&self, min_score: f64) -> Vec<SerializedPredicate> {
        self.address_scores
            .values()
            .filter(|p| p.score > min_score)
            .map(|p| p.to_serialzed())
            .collect()
    }

    fn print_scores(scores: &Vec<Predicate>, filter_scores: bool) {
        for predicate in scores.iter() {
            if filter_scores && predicate.score <= 0.5 {
                continue;
            }
            println!(
                "{:#x};{} ({})",
                predicate.address,
                predicate.score,
                predicate.to_string()
            );
        }
    }

    pub fn any_instruction_at_address_contains_reg(
        &self,
        address: usize,
        reg_index: usize,
    ) -> bool {
        self.crashes
            .0
            .par_iter()
            .chain(self.non_crashes.0.par_iter())
            .any(|t| match t.instructions.get(&address) {
                Some(instruction) => instruction.registers_min.get(reg_index).is_some(),
                _ => false,
            })
    }

    pub fn get_any_mnemonic(&self, address: usize) -> String {
        self.iter_all_instructions_at_address(address)
            .nth(0)
            .unwrap()
            .mnemonic
            .to_string()
    }
}
