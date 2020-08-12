use std::str::FromStr;

use nix::libc::user_regs_struct;

use crate::register::{Register, RegisterValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessSize {
    Size1Byte = 1,
    Size2Byte = 2,
    Size4Byte = 4,
    Size8Byte = 8,
}

impl FromStr for AccessSize {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "1" => Self::Size1Byte,
            "2" => Self::Size2Byte,
            "4" => Self::Size4Byte,
            "8" => Self::Size8Byte,
            _ => return Err(()),
        })
    }
}

impl Default for AccessSize {
    fn default() -> Self {
        Self::Size8Byte
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Memory(MemoryLocation),
    Register(Register),
    Immediate(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryLocation {
    pub offset: Option<isize>,
    pub base: Option<Register>,
    pub index: Option<(Register, ArraySize)>,
    // pub access_size: AccessSize,
}

impl MemoryLocation {
    pub fn address(&self, registers: &user_regs_struct) -> usize {
        let address = self
            .base
            .and_then(|reg| Some(reg.value(registers)))
            .unwrap_or(0)
            + self
                .index
                .and_then(|(reg, size)| Some(reg.value(registers) * size as usize))
                .unwrap_or(0);

        match self.offset {
            Some(offset) => {
                if offset >= 0 {
                    address + offset.abs() as usize
                } else {
                    address - offset.abs() as usize
                }
            }
            None => address,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArraySize {
    Size1Byte = 1,
    Size2Byte = 2,
    Size4Byte = 4,
    Size8Byte = 8,
}

impl FromStr for ArraySize {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "1" => Self::Size1Byte,
            "2" => Self::Size2Byte,
            "4" => Self::Size4Byte,
            "8" => Self::Size8Byte,
            _ => return Err(()),
        })
    }
}

impl Default for ArraySize {
    fn default() -> Self {
        Self::Size1Byte
    }
}

use nom::bytes::complete::tag;
use nom::character::complete::{alphanumeric1, digit1, hex_digit1};
use nom::{
    call, complete, do_parse, map, map_res, named, opt, peek, preceded, separated_list, switch,
    tag, take, take_while, verify,
};

named!(pub operands(&str) -> Vec<Operand>,
    separated_list!(tag(","), complete!(call!(operand)))
);

named!(operand(&str) -> Operand,
    do_parse!(
        take_while!(is_space) >>
        operand: switch!(peek!(take!(1)),
            "$" => call!(immediate) |
            "%" => map!(call!(register), |register| Operand::Register(register)) |
            _ => map!(call!(memory), |memory| Operand::Memory(memory))
        ) >>
        (operand)
    )
);

named!(immediate(&str) -> Operand,
    do_parse!(
        tag!("$") >>
        num: call!(address) >>
        (Operand::Immediate(num))
    )
);

named!(register(&str) -> Register,
    do_parse!(
        tag!("%") >>
        reg: map_res!(
            alphanumeric1,
            |name: &str| Register::from_str(name)
        ) >>
        (reg)
    )
);

named!(memory(&str) -> MemoryLocation,
    verify!(memory_empty,
        |memory: &MemoryLocation| memory.offset.is_some() || memory.base.is_some() || memory.index.is_some()
    )
);

named!(memory_empty(&str) -> MemoryLocation,
    do_parse!(
        offset: opt!(call!(address)) >>
        inner: opt!(call!(memory_inner)) >>
        (MemoryLocation {
            offset: offset.and_then(|offset| Some(offset as isize)),
            base: inner.and_then(|inner| inner.0),
            index: inner.and_then(|inner| inner.1),
        })
    )
);

named!(memory_inner(&str) -> (Option<Register>, Option<(Register, ArraySize)>),
    complete!(do_parse!(
        tag!("(") >>
        take_while!(is_space) >>
        base: opt!(call!(register)) >>
        index: opt!(call!(memory_index)) >>
        take_while!(is_space) >>
        tag!(")") >>
        ((base, index))
    ))
);

named!(memory_index(&str) -> (Register, ArraySize),
    do_parse!(
        tag!(",") >>
        take_while!(is_space) >>
        index: call!(register) >>
        scale: opt!(call!(memory_index_scale)) >>
        ((index, scale.unwrap_or(ArraySize::Size1Byte)))
    )
);

named!(memory_index_scale(&str) -> ArraySize,
    do_parse!(
        tag!(",") >>
        take_while!(is_space) >>
        scale: map_res!(digit1, |size| ArraySize::from_str(size)) >>
        (scale)
    )
);

named!(address(&str) -> usize,
    do_parse!(
        neg: opt!(tag!("-")) >>
        num: switch!(peek!(take!(2)),
            "0x" => call!(hex_number) |
            _ => call!(number)
        ) >>
        (match neg {
            Some(_) => -(num as isize) as usize,
            None => num
        })
    )
);

named!(number(&str) -> usize,
    map_res!(
        digit1,
        |num: &str| num.parse::<usize>()
    )
);
named!(hex_number(&str) -> usize,
    preceded!(tag!("0x"),
        map_res!(
            hex_digit1,
            |num: &str| usize::from_str_radix(num, 16)
        )
    )
);

#[inline]
pub fn is_space(chr: char) -> bool {
    chr == ' ' || chr == '\t'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::register::*;

    macro_rules! parse_error {
        ( $func:ident, $input:expr ) => {
            print!("parsing {:?}", $input);
            let result = $func($input);
            println!(" -> {:?}", result);
            assert!(result.is_err());
        };
    }
    macro_rules! parse_eq_inner {
        ( $func:ident, $input:expr, $output:expr, $expected:expr ) => {
            print!("parsing {:?}", $input);
            let result = $func($input);
            println!(" -> {:?}", result);
            assert_eq!(result, Ok(($output, $expected)));
        };
    }
    macro_rules! parse_eq {
        ( $func:ident, $input:expr, $expected:expr ) => {
            parse_eq_inner!($func, $input, "", $expected);
        };
    }

    #[test]
    fn test_operands() {
        parse_eq!(operands, "", vec![]);
        parse_eq!(
            operands,
            "%eax, -0x127c(%rbp)",
            vec![
                Operand::Register(Register32::Eax.into()),
                Operand::Memory(MemoryLocation {
                    offset: Some(-0x127c),
                    base: Some(Register64::Rbp.into()),
                    index: None,
                })
            ]
        );
    }

    #[test]
    fn test_memory_operand() {
        parse_error!(operand, "(%ecx,2)");
        parse_error!(operand, "(%ebx,%ecx,-1)");
        parse_error!(operand, "(%ebx,%ecx,3)");
        parse_error!(operand, "(%ebx,%ecx,0x8)");

        parse_eq!(
            operand,
            "0x42(%rsi,%ebx,4)",
            Operand::Memory(MemoryLocation {
                offset: Some(0x42),
                base: Some(Register64::Rsi.into()),
                index: Some((Register32::Ebx.into(), ArraySize::Size4Byte)),
            })
        );
        parse_eq!(
            operand,
            "(%rax, %rcx, 8)",
            Operand::Memory(MemoryLocation {
                offset: None,
                base: Some(Register64::Rax.into()),
                index: Some((Register64::Rcx.into(), ArraySize::Size8Byte)),
            })
        );
        parse_eq!(
            operand,
            "-0x127c(%rbp)",
            Operand::Memory(MemoryLocation {
                offset: Some(-0x127c),
                base: Some(Register64::Rbp.into()),
                index: None,
            })
        );
        parse_eq!(
            operand,
            "(%esi,%rax)",
            Operand::Memory(MemoryLocation {
                offset: None,
                base: Some(Register32::Esi.into()),
                index: Some((Register64::Rax.into(), ArraySize::Size1Byte)),
            })
        );

        parse_eq!(
            operand,
            "0x1337",
            Operand::Memory(MemoryLocation {
                offset: Some(0x1337),
                base: None,
                index: None,
            })
        );
        parse_eq!(
            operand,
            "1337()",
            Operand::Memory(MemoryLocation {
                offset: Some(1337),
                base: None,
                index: None,
            })
        );
        parse_eq!(
            operand,
            "-0x42()",
            Operand::Memory(MemoryLocation {
                offset: Some(-0x42),
                base: None,
                index: None,
            })
        );
    }

    #[test]
    fn test_register_operand() {
        parse_error!(operand, "%abc");
        parse_error!(operand, "%0xrax");

        parse_eq!(operand, "%rax", Operand::Register(Register64::Rax.into()));
        parse_eq!(operand, "%ah", Operand::Register(Register8High::Ah.into()));
    }

    #[test]
    fn test_immediate_operand() {
        parse_error!(operand, "$+1");
        parse_error!(operand, "$--1");

        parse_eq!(operand, "$0x42", Operand::Immediate(0x42));
        parse_eq!(operand, "$-1337", Operand::Immediate(-1337isize as usize));
    }
}
