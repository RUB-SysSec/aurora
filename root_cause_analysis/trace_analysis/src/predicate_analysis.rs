use crate::predicate_builder::PredicateBuilder;
use crate::predicates::Predicate;

use crate::trace_analyzer::TraceAnalyzer;
use rayon::prelude::*;

pub struct PredicateAnalyzer {}

impl PredicateAnalyzer {
    pub fn evaluate_best_predicate_at_address(
        address: usize,
        trace_analyzer: &TraceAnalyzer,
    ) -> Predicate {
        let predicates = PredicateBuilder::gen_predicates(address, trace_analyzer);

        if predicates.is_empty() {
            return Predicate::gen_empty(address);
        }

        let mut ret: Vec<Predicate> = predicates
            .into_par_iter()
            .map(|p| PredicateAnalyzer::evaluate_predicate(trace_analyzer, p))
            .collect();

        ret.sort_by(|p1, p2| p1.score.partial_cmp(&p2.score).unwrap());
        ret.pop().unwrap()
    }

    fn evaluate_predicate(trace_analyzer: &TraceAnalyzer, mut predicate: Predicate) -> Predicate {
        let true_positives = trace_analyzer
            .crashes
            .as_slice()
            .par_iter()
            .map(|t| t.instructions.get(&predicate.address))
            .filter(|i| predicate.execute(i))
            .count() as f64
            / trace_analyzer.crashes.len() as f64;
        let true_negatives = trace_analyzer
            .non_crashes
            .as_slice()
            .par_iter()
            .map(|t| t.instructions.get(&predicate.address))
            .filter(|i| !predicate.execute(i))
            .count() as f64
            / trace_analyzer.non_crashes.len() as f64;

        predicate.score = (true_positives + true_negatives) / 2.0;

        predicate
    }
}
