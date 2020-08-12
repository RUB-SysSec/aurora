use crate::trace::REGISTERS;
use crate::trace_analyzer::TraceAnalyzer;
use std::collections::HashSet;

pub struct TraceIntegrityChecker {}

impl TraceIntegrityChecker {
    pub fn check_traces(trace_analyzer: &TraceAnalyzer) {
        TraceIntegrityChecker::cfg_empty(trace_analyzer);
        TraceIntegrityChecker::cfg_heads(trace_analyzer);
        TraceIntegrityChecker::cfg_leaves(trace_analyzer);
        TraceIntegrityChecker::cfg_head_equals_first_instruction(trace_analyzer);
        TraceIntegrityChecker::cfg_addresses_unique(trace_analyzer);
        TraceIntegrityChecker::instruction_mnemonic_not_empty(trace_analyzer);
        TraceIntegrityChecker::compare_reg_min_last_max(trace_analyzer);
        TraceIntegrityChecker::untracked_memory_write(trace_analyzer);
    }

    fn cfg_empty(trace_analyzer: &TraceAnalyzer) {
        // cfg is not empty
        if trace_analyzer.cfg.is_empty() {
            println!("[E] CFG is empty");
        }
    }

    fn cfg_heads(trace_analyzer: &TraceAnalyzer) {
        let cfg_heads = trace_analyzer.cfg.heads();
        // there is only one cfg head (joint cfg of crashes and non-crashes)
        if cfg_heads.len() != 1 {
            println!("[E] CFG has {} heads (should have 1)", cfg_heads.len());
        }
    }

    fn cfg_leaves(trace_analyzer: &TraceAnalyzer) {
        // there is only one cfg exit
        // this assumption might not hold every time (crashes may have different leaves from non-crashes)
        if trace_analyzer.cfg.leaves().len() != 1 {
            println!(
                "[W] CFG has {} leaves (Should have 1 leaf unless Crash-CFG leaf != CFG leaf)",
                trace_analyzer.cfg.leaves().len()
            );
        }
    }

    fn cfg_head_equals_first_instruction(trace_analyzer: &TraceAnalyzer) {
        let head = trace_analyzer.cfg.heads().pop().unwrap();
        for trace in trace_analyzer.iter_all_traces() {
            if head != trace.first_address {
                println!("[E] CFG head (0x{:x}) is not equal to first instruction address (0x{:x}) reported in trace {}. Not re-running this check", head, trace.first_address, trace.name);
                return;
            }
        }
    }

    fn cfg_addresses_unique(trace_analyzer: &TraceAnalyzer) {
        let cfg_addresses: Vec<usize> = trace_analyzer
            .cfg
            .bbs()
            .flat_map(|b| b.body.iter())
            .cloned()
            .collect();

        let cfg_addresses_unique = cfg_addresses.iter().cloned().collect::<HashSet<_>>();
        let address_union = trace_analyzer.address_union();

        if cfg_addresses.len() != cfg_addresses_unique.len() {
            println!(
                "[E] #addresses ({}) !=  #unique_addresses ({}) in CFG",
                cfg_addresses.len(),
                cfg_addresses_unique.len()
            );
        }

        if cfg_addresses.len() != address_union.len() {
            println!(
                "[E] #addresses ({}) in CFG !=  #crash_address_union ({})",
                cfg_addresses.len(),
                address_union.len()
            );
        }
    }

    fn instruction_mnemonic_not_empty(trace_analyzer: &TraceAnalyzer) {
        // instruction mnemonic != ""
        for trace in trace_analyzer.iter_all_traces() {
            trace.instructions.values().for_each(|i| {
                if i.mnemonic == "".to_string() {
                    println!("[E] Instruction {:x} has empty mnemonic in trace {}. Not re-running this check",
                    i.address,
                    trace.name);
                    return;
                }
            });
        }
    }

    fn untracked_memory_write(trace_analyzer: &TraceAnalyzer) {
        for trace in trace_analyzer.iter_all_traces() {
            for instruction in trace.instructions.values() {
                if instruction.mnemonic.contains("], ")
                    && instruction.mnemonic.contains("mov")
                    && !instruction.mnemonic.contains("rep")
                {
                    if !instruction.registers_min.get(23).is_some() {
                        println!("[E] Memory write found in mnemonic but no memory address field tracked for instruction {:x} with mnemonic {} in trace {}. Not re-running this check",
                        instruction.address,
                        instruction.mnemonic,
                        trace.name);
                    }
                    if !instruction.registers_min.get(24).is_some() {
                        println!("[E] Memory write found in mnemonic but no memory value field tracked for instruction {:x} with mnemonic {} in trace {}. Not re-running this check",
                        instruction.address,
                        instruction.mnemonic,
                        trace.name);
                    }
                }
            }
        }
    }

    fn compare_reg_min_last_max(trace_analyzer: &TraceAnalyzer) {
        // reg_min <= reg_last <= reg_max
        for trace in trace_analyzer.iter_all_traces() {
            for instruction in trace.instructions.values() {
                (0..REGISTERS.len())
                    .into_iter()
                    .filter(|i| instruction.registers_min.get(*i).is_some())
                    .for_each(|i| {
                        let reg_min = instruction.registers_min.get(i).unwrap();
                        let reg_max = instruction.registers_max.get(i).unwrap();
//                        let reg_last = instruction.registers_last.get(i).unwrap();

                        if !(reg_min.value() <= reg_max.value()) {
                            println!("[E] min reg {} is not <= max reg for instruction {:x} in trace {}. Not re-running this check",
                            REGISTERS[i],
                            instruction.address,
                            trace.name);
                            return;
                        }

                    });
            }
        }
    }
}
