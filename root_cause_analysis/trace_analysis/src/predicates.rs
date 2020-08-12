use crate::trace::{Instruction, Register, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedPredicate {
    pub name: String,
    pub score: f64,
    pub address: usize,
}

impl SerializedPredicate {
    pub fn new(name: String, address: usize, score: f64) -> SerializedPredicate {
        SerializedPredicate {
            name,
            score,
            address,
        }
    }

    pub fn to_string(&self) -> String {
        format!("{:#018x} -- {} -- {}", self.address, self.name, self.score)
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string(&self).expect(&format!(
            "Could not serialize predicate {}",
            self.to_string()
        ))
    }
}

#[derive(Clone)]
pub struct Predicate {
    pub name: String,
    p1: Option<usize>,
    p2: Option<usize>,
    function: fn(&Instruction, Option<usize>, Option<usize>) -> bool,
    pub score: f64,
    pub address: usize,
}

impl Predicate {
    pub fn new(
        name: &str,
        address: usize,
        function: fn(&Instruction, Option<usize>, Option<usize>) -> bool,
        p1: Option<usize>,
        p2: Option<usize>,
    ) -> Predicate {
        Predicate {
            name: name.to_string(),
            address,
            p1,
            p2,
            function,
            score: 0.0,
        }
    }

    pub fn serialize(&self) -> String {
        let serialized = SerializedPredicate::new(self.name.to_string(), self.address, self.score);
        serde_json::to_string(&serialized).unwrap()
    }

    pub fn to_serialzed(&self) -> SerializedPredicate {
        SerializedPredicate::new(self.name.to_string(), self.address, self.score)
    }

    pub fn execute(&self, instruction_option: &Option<&Instruction>) -> bool {
        match instruction_option {
            Some(instruction) => (self.function)(instruction, self.p1, self.p2),
            None => false,
        }
    }

    pub fn gen_empty(address: usize) -> Predicate {
        Predicate::new("empty", address, empty, None, None)
    }

    pub fn to_string(&self) -> String {
        format!("{}", self.name)
    }
}

pub fn empty(_: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    false
}

pub fn is_visited(_: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    true
}

pub fn selector_val_less_name(selector: &Selector) -> String {
    match selector {
        Selector::RegMin => format!("min_reg_val_less"),
        Selector::RegMax => format!("max_reg_val_less"),
        Selector::RegMaxMinDiff => format!("max_min_diff_reg_val_less"),
        Selector::InsCount => format!("ins_count_less"),
        _ => unreachable!(),
    }
}

pub fn selector_val_less(
    selector: &Selector,
) -> fn(&Instruction, Option<usize>, Option<usize>) -> bool {
    match selector {
        Selector::RegMin => min_reg_val_less,
        Selector::RegMax => max_reg_val_less,
        Selector::RegMaxMinDiff => max_min_diff_reg_val_less,
        //        Selector::InsCount => ins_count_less,
        _ => unreachable!(),
    }
}

pub fn min_reg_val_less(
    instruction: &Instruction,
    reg_index: Option<usize>,
    value: Option<usize>,
) -> bool {
    match instruction.registers_min.get(reg_index.unwrap()) {
        Some(reg) => reg.value() < value.unwrap() as u64,
        None => false,
    }
}

pub fn max_reg_val_less(
    instruction: &Instruction,
    reg_index: Option<usize>,
    value: Option<usize>,
) -> bool {
    match instruction.registers_max.get(reg_index.unwrap()) {
        Some(reg) => reg.value() < value.unwrap() as u64,
        None => false,
    }
}

pub fn max_min_diff_reg_val_less(
    instruction: &Instruction,
    reg_index: Option<usize>,
    value: Option<usize>,
) -> bool {
    match (
        instruction.registers_max.get(reg_index.unwrap()),
        instruction.registers_min.get(reg_index.unwrap()),
    ) {
        (Some(reg_max), Some(reg_min)) => reg_max.value() - reg_min.value() < value.unwrap() as u64,
        _ => false,
    }
}

pub fn selector_val_greater_or_equal_name(selector: &Selector) -> String {
    match selector {
        Selector::RegMin => format!("min_reg_val_greater_or_equal"),
        Selector::RegMax => format!("max_reg_val_greater_or_equal"),
        Selector::RegMaxMinDiff => format!("max_min_diff_reg_val_greater_or_equal"),
        Selector::InsCount => format!("ins_count_greater_or_equal"),
        _ => unreachable!(),
    }
}

pub fn selector_val_greater_or_equal(
    selector: &Selector,
) -> fn(&Instruction, Option<usize>, Option<usize>) -> bool {
    match selector {
        Selector::RegMin => min_reg_val_greater_or_equal,
        Selector::RegMax => max_reg_val_greater_or_equal,
        Selector::RegMaxMinDiff => max_min_diff_reg_val_greater_or_equal,
        //        Selector::InsCount => ins_count_greater_or_equal,
        _ => unreachable!(),
    }
}

pub fn min_reg_val_greater_or_equal(
    instruction: &Instruction,
    reg_index: Option<usize>,
    value: Option<usize>,
) -> bool {
    match instruction.registers_min.get(reg_index.unwrap()) {
        Some(reg) => reg.value() >= value.unwrap() as u64,
        None => false,
    }
}

pub fn max_reg_val_greater_or_equal(
    instruction: &Instruction,
    reg_index: Option<usize>,
    value: Option<usize>,
) -> bool {
    match instruction.registers_max.get(reg_index.unwrap()) {
        Some(reg) => reg.value() >= value.unwrap() as u64,
        None => false,
    }
}

pub fn max_min_diff_reg_val_greater_or_equal(
    instruction: &Instruction,
    reg_index: Option<usize>,
    value: Option<usize>,
) -> bool {
    match (
        instruction.registers_max.get(reg_index.unwrap()),
        instruction.registers_min.get(reg_index.unwrap()),
    ) {
        (Some(reg_max), Some(reg_min)) => {
            reg_max.value() - reg_min.value() >= value.unwrap() as u64
        }
        _ => false,
    }
}

fn is_flag_bit_set(instruction: &Instruction, reg_type: Selector, pos: u64) -> bool {
    match reg_type {
        Selector::RegMin => is_reg_bit_set(instruction.registers_min.get(22), pos),
        Selector::RegMax => is_reg_bit_set(instruction.registers_max.get(22), pos),
        //        Selector::RegLast => is_reg_bit_set(instruction.registers_last.get(22), pos),
        _ => unreachable!(),
    }
}

fn is_reg_bit_set(reg: Option<&Register>, pos: u64) -> bool {
    match reg.is_some() {
        true => match reg.unwrap().value() & (1 << pos) {
            0 => false,
            _ => true,
        },
        _ => false,
    }
}

pub fn min_carry_flag_set(instruction: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    is_flag_bit_set(instruction, Selector::RegMin, 0)
}

pub fn min_parity_flag_set(instruction: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    is_flag_bit_set(instruction, Selector::RegMin, 2)
}

pub fn min_adjust_flag_set(instruction: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    is_flag_bit_set(instruction, Selector::RegMin, 4)
}

pub fn min_zero_flag_set(instruction: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    is_flag_bit_set(instruction, Selector::RegMin, 6)
}

pub fn min_sign_flag_set(instruction: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    is_flag_bit_set(instruction, Selector::RegMin, 7)
}

pub fn min_trap_flag_set(instruction: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    is_flag_bit_set(instruction, Selector::RegMin, 8)
}

pub fn min_interrupt_flag_set(
    instruction: &Instruction,
    _: Option<usize>,
    _: Option<usize>,
) -> bool {
    is_flag_bit_set(instruction, Selector::RegMin, 9)
}

pub fn min_direction_flag_set(
    instruction: &Instruction,
    _: Option<usize>,
    _: Option<usize>,
) -> bool {
    is_flag_bit_set(instruction, Selector::RegMin, 10)
}

pub fn min_overflow_flag_set(
    instruction: &Instruction,
    _: Option<usize>,
    _: Option<usize>,
) -> bool {
    is_flag_bit_set(instruction, Selector::RegMin, 11)
}

pub fn max_carry_flag_set(instruction: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    is_flag_bit_set(instruction, Selector::RegMax, 0)
}

pub fn max_parity_flag_set(instruction: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    is_flag_bit_set(instruction, Selector::RegMax, 2)
}

pub fn max_adjust_flag_set(instruction: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    is_flag_bit_set(instruction, Selector::RegMax, 4)
}

pub fn max_zero_flag_set(instruction: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    is_flag_bit_set(instruction, Selector::RegMax, 6)
}

pub fn max_sign_flag_set(instruction: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    is_flag_bit_set(instruction, Selector::RegMax, 7)
}

pub fn max_trap_flag_set(instruction: &Instruction, _: Option<usize>, _: Option<usize>) -> bool {
    is_flag_bit_set(instruction, Selector::RegMax, 8)
}

pub fn max_interrupt_flag_set(
    instruction: &Instruction,
    _: Option<usize>,
    _: Option<usize>,
) -> bool {
    is_flag_bit_set(instruction, Selector::RegMax, 9)
}

pub fn max_direction_flag_set(
    instruction: &Instruction,
    _: Option<usize>,
    _: Option<usize>,
) -> bool {
    is_flag_bit_set(instruction, Selector::RegMax, 10)
}

pub fn max_overflow_flag_set(
    instruction: &Instruction,
    _: Option<usize>,
    _: Option<usize>,
) -> bool {
    is_flag_bit_set(instruction, Selector::RegMax, 11)
}

pub fn num_successors_greater(
    instruction: &Instruction,
    n: Option<usize>,
    _: Option<usize>,
) -> bool {
    instruction.successors.len() > n.unwrap()
}

pub fn num_successors_equal(instruction: &Instruction, n: Option<usize>, _: Option<usize>) -> bool {
    instruction.successors.len() == n.unwrap()
}

pub fn has_edge_to(instruction: &Instruction, address: Option<usize>, _: Option<usize>) -> bool {
    instruction
        .successors
        .iter()
        .any(|s| s.address == address.unwrap())
}

pub fn edge_only_taken_to(
    instruction: &Instruction,
    address: Option<usize>,
    _: Option<usize>,
) -> bool {
    instruction
        .successors
        .iter()
        .any(|s| s.address == address.unwrap())
        && instruction.successors.len() == 1
}
