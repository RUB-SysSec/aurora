use nix::libc::user_regs_struct;
use std::str::FromStr;

use crate::register::{Register, RegisterValue};
use crate::rflags::RFlags;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    Compare(ComparePredicate),
    Edge(EdgePredicate),
    FlagSet(RFlags),
    Visited,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparePredicate {
    pub destination: ValueDestination,
    pub compare: Compare,
    pub value: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueDestination {
    Address(MemoryLocation),
    Memory(AccessSize, MemoryLocation),
    Register(Register),
}

impl ValueDestination {
    pub fn register(register: Register) -> Self {
        Self::Register(register)
    }
}

pub type AccessSize = u8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryLocation {
    segment: Option<Register>,
    base: Option<Register>,
    index: Option<Register>,
    scale: u8,
    displacement: Option<i64>,
}

impl MemoryLocation {
    fn from_memory_info(mem: &zydis::ffi::MemoryInfo) -> Self {
        Self {
            segment: Register::from_zydis_register(mem.segment),
            base: Register::from_zydis_register(mem.base),
            index: Register::from_zydis_register(mem.index),
            scale: mem.scale,
            displacement: if mem.disp.has_displacement {
                Some(mem.disp.displacement)
            } else {
                None
            },
        }
    }
}

impl MemoryLocation {
    pub fn address(&self, registers: &user_regs_struct) -> usize {
        let address = self
            .base
            .and_then(|reg| Some(reg.value(registers)))
            .unwrap_or(0)
            + self
                .index
                .and_then(|reg| Some(reg.value(registers) * self.scale as usize))
                .unwrap_or(0);

        match self.displacement {
            Some(displacement) => {
                if displacement >= 0 {
                    address + displacement.abs() as usize
                } else {
                    address - displacement.abs() as usize
                }
            }
            None => address,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compare {
    Less,
    Greater,
    GreaterOrEqual,
    Equal,
    NotEqual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgePredicate {
    pub source: usize,
    pub transition: EdgeTransition,
    pub destination: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeTransition {
    Taken,
    NotTaken,
}

pub fn convert_predicate(
    predicate: &str,
    instruction: zydis::DecodedInstruction,
) -> Option<Predicate> {
    let parts: Vec<_> = predicate.split(' ').collect();
    let function = match parts.len() {
        1 | 2 => parts[0],
        3 => parts[1],
        _ => unimplemented!(),
    };

    if function.contains("edge") {
        let source = usize::from_str_radix(&parts[0][2..], 16).expect("failed to parse source");
        let destination =
            usize::from_str_radix(&parts[2][2..], 16).expect("failed to parse destination");
        let transition = match function {
            "has_edge_to" => EdgeTransition::Taken,
            "edge_only_taken_to" => EdgeTransition::NotTaken,
            "last_edge_to" => return None,
            _ => unimplemented!(),
        };

        return Some(Predicate::Edge(EdgePredicate {
            source,
            transition,
            destination,
        }));
    } else if function.contains("reg_val") {
        let value = usize::from_str_radix(&parts[2][2..], 16).expect("failed to parse value");
        let memory_locations = instruction.operands[..instruction.operand_count as usize]
            .into_iter()
            .filter(|op| match op.ty {
                zydis::OperandType::MEMORY => true,
                _ => false,
            });
        let memory = memory_locations
            .last()
            .and_then(|op| Some(MemoryLocation::from_memory_info(&op.mem)));

        let destination = match parts[0] {
            "memory_address" => ValueDestination::Address(memory.expect("no memory location")),
            "memory_value" => ValueDestination::Memory(
                instruction.operand_width,
                memory.expect("no memory location"),
            ),

            "seg_cs" => return None,
            "seg_ss" => return None,
            "seg_ds" => return None,
            "seg_es" => return None,
            "seg_fs" => return None,
            "seg_gs" => return None,

            "eflags" => return None,

            register => ValueDestination::Register(
                Register::from_str(register).expect("failed to parse register"),
            ),
        };

        let compare = match function {
            "min_reg_val_less" => Compare::Less,
            "max_reg_val_less" => Compare::Less,
            "last_reg_val_less" => return None,
            "max_min_diff_reg_val_less" => return None,

            "min_reg_val_greater_or_equal" => Compare::GreaterOrEqual,
            "max_reg_val_greater_or_equal" => Compare::GreaterOrEqual,
            "last_reg_val_greater_or_equal" => return None,
            "max_min_diff_reg_val_greater_or_equal" => return None,

            _ => unimplemented!(),
        };

        return Some(Predicate::Compare(ComparePredicate {
            destination,
            compare,
            value,
        }));
    } else if function.contains("ins_count") {
        // "ins_count_less"
        // "ins_count_greater_or_equal"
    } else if function.contains("selector_val") {
        // "selector_val_less_name"
        // "selector_val_less"
        // "selector_val_greater_or_equal_name"
        // "selector_val_greater_or_equal"
    } else if function.contains("num_successors") {
        // "num_successors_greater" =>
        // "num_successors_equal" =>
    } else if function.contains("flag") {
        let flag = match function {
            "min_carry_flag_set" => RFlags::CARRY_FLAG,
            "min_parity_flag_set" => RFlags::PARITY_FLAG,
            "min_adjust_flag_set" => RFlags::AUXILIARY_CARRY_FLAG,
            "min_zero_flag_set" => RFlags::ZERO_FLAG,
            "min_sign_flag_set" => RFlags::SIGN_FLAG,
            "min_trap_flag_set" => RFlags::TRAP_FLAG,
            "min_interrupt_flag_set" => RFlags::INTERRUPT_FLAG,
            "min_direction_flag_set" => RFlags::DIRECTION_FLAG,
            "min_overflow_flag_set" => RFlags::OVERFLOW_FLAG,

            "max_carry_flag_set" => RFlags::CARRY_FLAG,
            "max_parity_flag_set" => RFlags::PARITY_FLAG,
            "max_adjust_flag_set" => RFlags::AUXILIARY_CARRY_FLAG,
            "max_zero_flag_set" => RFlags::ZERO_FLAG,
            "max_sign_flag_set" => RFlags::SIGN_FLAG,
            "max_trap_flag_set" => RFlags::TRAP_FLAG,
            "max_interrupt_flag_set" => RFlags::INTERRUPT_FLAG,
            "max_direction_flag_set" => RFlags::DIRECTION_FLAG,
            "max_overflow_flag_set" => RFlags::OVERFLOW_FLAG,

            "last_carry_flag_set" => return None,
            "last_parity_flag_set" => return None,
            "last_adjust_flag_set" => return None,
            "last_zero_flag_set" => return None,
            "last_sign_flag_set" => return None,
            "last_trap_flag_set" => return None,
            "last_interrupt_flag_set" => return None,
            "last_direction_flag_set" => return None,
            "last_overflow_flag_set" => return None,

            _ => unimplemented!(),
        };

        return Some(Predicate::FlagSet(flag));
    } else if function == "is_visited" {
        return Some(Predicate::Visited);
    } else {
        log::error!("unknown predicate function {:?}", function);
        unimplemented!()
    }

    None
}
