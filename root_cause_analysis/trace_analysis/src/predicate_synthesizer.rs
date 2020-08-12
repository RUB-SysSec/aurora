use crate::predicates::*;
use crate::trace::{Selector, REGISTERS};
use crate::trace_analyzer::TraceAnalyzer;
use rayon::prelude::*;

pub struct PredicateSynthesizer {}

pub fn gen_reg_val_name(reg_index: Option<usize>, pred_name: String, value: u64) -> String {
    match reg_index.is_some() {
        true => format!(
            "{} {} 0x{:x}",
            REGISTERS[reg_index.unwrap()],
            pred_name,
            value
        ),
        false => format!("{} {}", pred_name, value),
    }
}

impl PredicateSynthesizer {
    pub fn constant_predicates_at_address(
        address: usize,
        trace_analyzer: &TraceAnalyzer,
    ) -> Vec<Predicate> {
        let mut predicates = vec![];

        predicates.extend(
            PredicateSynthesizer::register_constant_predicates_at_address(
                address,
                trace_analyzer,
                &Selector::RegMax,
            ),
        );
        predicates.extend(
            PredicateSynthesizer::register_constant_predicates_at_address(
                address,
                trace_analyzer,
                &Selector::RegMin,
            ),
        );

        predicates
    }

    fn register_constant_predicates_at_address(
        address: usize,
        trace_analyzer: &TraceAnalyzer,
        selector: &Selector,
    ) -> Vec<Predicate> {
        (0..REGISTERS.len())
            .into_par_iter()
            .filter(|reg_index| {
                trace_analyzer.any_instruction_at_address_contains_reg(address, *reg_index)
            })
            /* skip RSP */
            .filter(|reg_index| *reg_index != 7)
            /* skip EFLAGS */
            .filter(|reg_index| *reg_index != 22)
            /* skip memory address */
            .filter(|reg_index| *reg_index != 23)
            /* skip all heap addresses */
            .filter(|reg_index| {
                !trace_analyzer
                    .values_at_address(address, selector, Some(*reg_index))
                    .into_iter()
                    .all(|v: u64| {
                        trace_analyzer.memory_addresses.heap_start <= v as usize
                            && v as usize <= trace_analyzer.memory_addresses.heap_end
                    })
            })
            /* skip all stack addresses */
            .filter(|reg_index| {
                !trace_analyzer
                    .values_at_address(address, selector, Some(*reg_index))
                    .into_iter()
                    .all(|v: u64| {
                        trace_analyzer.memory_addresses.stack_start <= v as usize
                            && v as usize <= trace_analyzer.memory_addresses.stack_end
                    })
            })
            .flat_map(|reg_index| {
                PredicateSynthesizer::synthesize_constant_predicates(
                    address,
                    trace_analyzer,
                    selector,
                    Some(reg_index),
                )
            })
            .collect()
    }

    fn synthesize_constant_predicates(
        address: usize,
        trace_analyzer: &TraceAnalyzer,
        selector: &Selector,
        reg_index: Option<usize>,
    ) -> Vec<Predicate> {
        let values = trace_analyzer.unique_values_at_address(address, selector, reg_index);
        if values.is_empty() {
            return vec![];
        }

        let mut f: Vec<_> = values
            .par_iter()
            .map(|v| {
                (
                    v,
                    PredicateSynthesizer::evaluate_value_at_address(
                        address,
                        trace_analyzer,
                        selector,
                        reg_index,
                        *v,
                    ),
                )
            })
            .collect();

        f.sort_by(|(_, f1), (_, f2)| f1.partial_cmp(&f2).unwrap());

        PredicateSynthesizer::build_constant_predicates(
            address,
            selector,
            reg_index,
            PredicateSynthesizer::arithmetic_mean(*f.first().unwrap().0, &values),
            PredicateSynthesizer::arithmetic_mean(*f.last().unwrap().0, &values),
        )
    }

    fn arithmetic_mean(v1: u64, values: &Vec<u64>) -> u64 {
        match values.iter().filter(|v| *v < &v1).max() {
            Some(v2) => ((v1 as f64 + *v2 as f64) / 2.0).round() as u64,
            None => v1,
        }
    }

    fn build_constant_predicates(
        address: usize,
        selector: &Selector,
        reg_index: Option<usize>,
        v1: u64,
        v2: u64,
    ) -> Vec<Predicate> {
        let pred_name1 =
            gen_reg_val_name(reg_index, selector_val_greater_or_equal_name(selector), v1);
        let pred_name2 = gen_reg_val_name(reg_index, selector_val_less_name(selector), v2);

        vec![
            Predicate::new(
                &pred_name1,
                address,
                selector_val_greater_or_equal(selector),
                reg_index,
                Some(v1 as usize),
            ),
            Predicate::new(
                &pred_name2,
                address,
                selector_val_less(selector),
                reg_index,
                Some(v2 as usize),
            ),
        ]
    }

    fn evaluate_value_at_address(
        address: usize,
        trace_analyzer: &TraceAnalyzer,
        selector: &Selector,
        reg_index: Option<usize>,
        val: u64,
    ) -> f64 {
        let pred_name = format!(
            "{:?} {} {}",
            reg_index,
            selector_val_less_name(selector),
            val
        );

        let predicate = Predicate::new(
            &pred_name,
            address,
            selector_val_less(selector),
            reg_index,
            Some(val as usize),
        );

        PredicateSynthesizer::evaluate_predicate_with_reachability(
            address,
            trace_analyzer,
            &predicate,
        )
    }

    pub fn evaluate_predicate_with_reachability(
        address: usize,
        trace_analyzer: &TraceAnalyzer,
        predicate: &Predicate,
    ) -> f64 {
        let true_positives = trace_analyzer
            .crashes
            .as_slice()
            .par_iter()
            .filter(|t| t.instructions.get(&address).is_some())
            .map(|t| t.instructions.get(&address))
            .filter(|i| predicate.execute(i))
            .count() as f64
            / trace_analyzer.crashes.len() as f64;
        let true_negatives = (trace_analyzer
            .non_crashes
            .as_slice()
            .par_iter()
            .filter(|t| t.instructions.get(&address).is_some())
            .map(|t| t.instructions.get(&address))
            .filter(|i| !predicate.execute(i))
            .count() as f64
            + trace_analyzer
                .non_crashes
                .as_slice()
                .par_iter()
                .filter(|t| t.instructions.get(&address).is_none())
                .count() as f64)
            / trace_analyzer.non_crashes.len() as f64;

        let score = (true_positives + true_negatives) / 2.0;

        score
    }
}
