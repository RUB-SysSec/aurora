use crate::config::Config;
use crate::predicate_analysis::PredicateAnalyzer;
use crate::trace::Trace;
use crate::trace_analyzer::TraceAnalyzer;
use std::fs::File;
use std::io::Write;

pub fn diff_traces(config: &Config, trace_analyzer: &TraceAnalyzer) {
    let mut file = File::create(format!("{}/verbose_info.csv", config.output_directory)).unwrap();
    /* addresses that have been seen in traces AND non-traces */
    for addr in trace_analyzer.cfg.keys() {
        write_instruction_from_traces_at_address(
            &mut file,
            *addr,
            &trace_analyzer.crashes.as_slice(),
            "crash",
        );
        write_instruction_from_traces_at_address(
            &mut file,
            *addr,
            &trace_analyzer.non_crashes.as_slice(),
            "non_crash",
        );
    }
}

pub fn diff_traces_at_address(config: &Config, trace_analyzer: &TraceAnalyzer) {
    let mut file = File::create(format!("{}/verbose_info.csv", config.output_directory)).unwrap();
    write_instruction_from_traces_at_address(
        &mut file,
        config.dump_address,
        &trace_analyzer.crashes.as_slice(),
        "crash",
    );
    write_instruction_from_traces_at_address(
        &mut file,
        config.dump_address,
        &trace_analyzer.non_crashes.as_slice(),
        "non_crash",
    );
}

pub fn dump_trace_info(config: &Config, trace_analyzer: &TraceAnalyzer) {
    let mut file = File::create(format!("{}/trace_info.csv", config.output_directory)).unwrap();

    write_traces_info(&mut file, &trace_analyzer.crashes.as_slice(), "crash");

    write_traces_info(
        &mut file,
        &trace_analyzer.non_crashes.as_slice(),
        "non_crash",
    );
}

pub fn debug_predicate_at_address(address: usize, trace_analyzer: &TraceAnalyzer) {
    let predicate = PredicateAnalyzer::evaluate_best_predicate_at_address(address, trace_analyzer);

    println!(
        "0x{:x} -- {} -- {}",
        predicate.address,
        predicate.to_string(),
        predicate.score
    );
}

fn write_traces_info(file: &mut File, traces: &[Trace], flag: &str) {
    for trace in traces.iter() {
        write!(file, "{};{}\n", trace.to_string(), flag).unwrap();
    }
}

fn write_instruction_from_traces_at_address(
    file: &mut File,
    addr: usize,
    traces: &[Trace],
    flag: &str,
) {
    for trace in traces.iter().filter(|t| t.instructions.contains_key(&addr)) {
        write!(file, "{};{}\n", trace.instructions[&addr].to_string(), flag).unwrap();
    }
}
