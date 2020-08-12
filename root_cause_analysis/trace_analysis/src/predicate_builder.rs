use crate::control_flow_graph::ControlFlowGraph;
use crate::predicate_synthesizer::{gen_reg_val_name, PredicateSynthesizer};
use crate::predicates::*;
use crate::trace::Instruction;
use crate::trace::{Selector, REGISTERS};
use crate::trace_analyzer::TraceAnalyzer;

pub struct PredicateBuilder {}

impl PredicateBuilder {
    fn gen_visited(address: usize) -> Vec<Predicate> {
        vec![Predicate::new(
            "is_visited",
            address,
            is_visited,
            None,
            None,
        )]
    }
    fn gen_all_edge_from_to_predicates(
        address: usize,
        cfg: &ControlFlowGraph,
        pred_name: &str,
        func: fn(&Instruction, Option<usize>, Option<usize>) -> bool,
    ) -> Vec<Predicate> {
        cfg.get_instruction_successors(address)
            .iter()
            .map(|to| {
                let pred_name = format!("0x{:x} {} 0x{:x}", address, pred_name, to);
                Predicate::new(&pred_name, address, func, Some(*to), None)
            })
            .collect()
    }

    fn gen_all_edge_val_predicates(
        address: usize,
        pred_name: &str,
        value: usize,
        func: fn(&Instruction, Option<usize>, Option<usize>) -> bool,
    ) -> Predicate {
        let pred_name = format!("{} {}", pred_name, value);

        Predicate::new(&pred_name, address, func, Some(value), None)
    }

    pub fn gen_flag_predicates(address: usize, trace_analyzer: &TraceAnalyzer) -> Vec<Predicate> {
        if !trace_analyzer.any_instruction_at_address_contains_reg(address, 22) {
            return vec![];
        }

        vec![
            // min
            Predicate::new(
                "min_carry_flag_set",
                address,
                min_carry_flag_set,
                None,
                None,
            ),
            Predicate::new(
                "min_parity_flag_set",
                address,
                min_parity_flag_set,
                None,
                None,
            ),
            Predicate::new(
                "min_adjust_flag_set",
                address,
                min_adjust_flag_set,
                None,
                None,
            ),
            Predicate::new("min_zero_flag_set", address, min_zero_flag_set, None, None),
            Predicate::new("min_sign_flag_set", address, min_sign_flag_set, None, None),
            Predicate::new("min_trap_flag_set", address, min_trap_flag_set, None, None),
            Predicate::new(
                "min_interrupt_flag_set",
                address,
                min_interrupt_flag_set,
                None,
                None,
            ),
            Predicate::new(
                "min_direction_flag_set",
                address,
                min_direction_flag_set,
                None,
                None,
            ),
            Predicate::new(
                "min_overflow_flag_set",
                address,
                min_overflow_flag_set,
                None,
                None,
            ),
            // max
            Predicate::new(
                "max_carry_flag_set",
                address,
                max_carry_flag_set,
                None,
                None,
            ),
            Predicate::new(
                "max_parity_flag_set",
                address,
                max_parity_flag_set,
                None,
                None,
            ),
            Predicate::new(
                "max_adjust_flag_set",
                address,
                max_adjust_flag_set,
                None,
                None,
            ),
            Predicate::new("max_zero_flag_set", address, max_zero_flag_set, None, None),
            Predicate::new("max_sign_flag_set", address, max_sign_flag_set, None, None),
            Predicate::new("max_trap_flag_set", address, max_trap_flag_set, None, None),
            Predicate::new(
                "max_interrupt_flag_set",
                address,
                max_interrupt_flag_set,
                None,
                None,
            ),
            Predicate::new(
                "max_direction_flag_set",
                address,
                max_direction_flag_set,
                None,
                None,
            ),
            Predicate::new(
                "max_overflow_flag_set",
                address,
                max_overflow_flag_set,
                None,
                None,
            ),
        ]
    }

    pub fn gen_cfg_predicates(address: usize, cfg: &ControlFlowGraph) -> Vec<Predicate> {
        let mut ret = vec![];

        // check if end of basic block
        if !cfg.is_bb_end(address) {
            return ret;
        }

        // #successors > 0
        ret.push(PredicateBuilder::gen_all_edge_val_predicates(
            address,
            "num_successors_greater",
            0,
            num_successors_greater,
        ));
        // #successors > 1
        ret.push(PredicateBuilder::gen_all_edge_val_predicates(
            address,
            "num_successors_greater",
            1,
            num_successors_greater,
        ));
        // #successors > 2
        ret.push(PredicateBuilder::gen_all_edge_val_predicates(
            address,
            "num_successors_greater",
            2,
            num_successors_greater,
        ));

        // #successors == 0
        ret.push(PredicateBuilder::gen_all_edge_val_predicates(
            address,
            "num_successors_equal",
            0,
            num_successors_equal,
        ));
        // #successors == 1
        ret.push(PredicateBuilder::gen_all_edge_val_predicates(
            address,
            "num_successors_equal",
            1,
            num_successors_equal,
        ));
        // #successors == 2
        ret.push(PredicateBuilder::gen_all_edge_val_predicates(
            address,
            "num_successors_equal",
            2,
            num_successors_equal,
        ));
        // edge addr -> x cfg edges exists
        ret.extend(PredicateBuilder::gen_all_edge_from_to_predicates(
            address,
            cfg,
            "has_edge_to",
            has_edge_to,
        ));
        ret.extend(PredicateBuilder::gen_all_edge_from_to_predicates(
            address,
            cfg,
            "edge_only_taken_to",
            edge_only_taken_to,
        ));
        ret
    }

    pub fn gen_all_reg_val_predicates(
        address: usize,
        trace_analyzer: &TraceAnalyzer,
        selector: &Selector,
        value: usize,
    ) -> Vec<Predicate> {
        (0..REGISTERS.len())
            .into_iter()
            .filter(|reg_index| {
                trace_analyzer.any_instruction_at_address_contains_reg(address, *reg_index)
            })
            /* skip RSP */
            .filter(|reg_index| *reg_index != 7)
            /* skip EFLAGS */
            .filter(|reg_index| *reg_index != 22)
            /* skip memory address */
            .filter(|reg_index| *reg_index != 23)
            .map(|reg_index| {
                let pred_name = gen_reg_val_name(
                    Some(reg_index),
                    selector_val_less_name(selector),
                    value as u64,
                );
                Predicate::new(
                    &pred_name,
                    address,
                    selector_val_less(&selector),
                    Some(reg_index),
                    Some(value),
                )
            })
            .collect()
    }

    pub fn gen_register_predicates(
        address: usize,
        trace_analyzer: &TraceAnalyzer,
    ) -> Vec<Predicate> {
        let mut ret = vec![];

        ret.extend(PredicateBuilder::gen_all_reg_val_predicates(
            address,
            trace_analyzer,
            &Selector::RegMax,
            0xffffffffffffffff,
        ));
        ret.extend(PredicateBuilder::gen_all_reg_val_predicates(
            address,
            trace_analyzer,
            &Selector::RegMax,
            0xffffffff,
        ));
        ret.extend(PredicateBuilder::gen_all_reg_val_predicates(
            address,
            trace_analyzer,
            &Selector::RegMax,
            0xffff,
        ));
        ret.extend(PredicateBuilder::gen_all_reg_val_predicates(
            address,
            trace_analyzer,
            &Selector::RegMax,
            0xff,
        ));

        ret.extend(PredicateBuilder::gen_all_reg_val_predicates(
            address,
            trace_analyzer,
            &Selector::RegMin,
            0xffffffffffffffff,
        ));
        ret.extend(PredicateBuilder::gen_all_reg_val_predicates(
            address,
            trace_analyzer,
            &Selector::RegMin,
            0xffffffff,
        ));
        ret.extend(PredicateBuilder::gen_all_reg_val_predicates(
            address,
            trace_analyzer,
            &Selector::RegMin,
            0xffff,
        ));
        ret.extend(PredicateBuilder::gen_all_reg_val_predicates(
            address,
            trace_analyzer,
            &Selector::RegMin,
            0xff,
        ));

        ret
    }

    pub fn gen_predicates(address: usize, trace_analyzer: &TraceAnalyzer) -> Vec<Predicate> {
        let mut ret = vec![];

        let skip_register_predicates =
            PredicateBuilder::skip_register_mnemonic(trace_analyzer.get_any_mnemonic(address));

        ret.extend(PredicateBuilder::gen_visited(address));

        if !skip_register_predicates {
            ret.extend(PredicateSynthesizer::constant_predicates_at_address(
                address,
                trace_analyzer,
            ));

            ret.extend(PredicateBuilder::gen_register_predicates(
                address,
                &trace_analyzer,
            ));
        }

        ret.extend(PredicateBuilder::gen_cfg_predicates(
            address,
            &trace_analyzer.cfg,
        ));

        if !skip_register_predicates {
            ret.extend(PredicateBuilder::gen_flag_predicates(
                address,
                &trace_analyzer,
            ));
        }

        ret
    }

    fn skip_register_mnemonic(mnemonic: String) -> bool {
        match mnemonic.as_str() {
            // leave instruction
            _ if mnemonic.contains("leave") => true,
            // contains floating point register
            _ if mnemonic.contains("xmm") => true,
            // contains rsp but is no memory operation
            _ if !mnemonic.contains("[") && mnemonic.contains("rsp") => true,
            // moves a constant into register/memory
            _ if mnemonic.contains("mov") && mnemonic.contains(", 0x") => true,
            _ => false,
        }
    }
}
