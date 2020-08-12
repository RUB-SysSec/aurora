use serde::{Deserialize, Serialize};
use std::collections::hash_map::Keys;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;

pub static REGISTERS: [&str; 25] = [
    "rax",
    "rbx",
    "rcx",
    "rdx",
    "rsi",
    "rdi",
    "rbp",
    "rsp",
    "r8",
    "r9",
    "r10",
    "r11",
    "r12",
    "r13",
    "r14",
    "r15",
    "seg_cs",
    "seg_ss",
    "seg_ds",
    "seg_es",
    "seg_fs",
    "seg_gs",
    "eflags",
    "memory_address",
    "memory_value",
];

pub enum Selector {
    RegMin,
    RegMax,
    RegLast,
    RegMaxMinDiff,
    InsCount,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Register {
    value: u64,
}

impl Register {
    pub fn new(_: &str, value: u64) -> Register {
        Register { value }
    }

    pub fn value(&self) -> u64 {
        self.value
    }

    pub fn to_string(&self) -> String {
        format!("{:#018x}", self.value())
    }

    pub fn to_string_extended(&self) -> String {
        format!("{:#018x}", self.value())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Registers(HashMap<usize, Register>);

impl Registers {
    pub fn get(&self, index: usize) -> Option<&Register> {
        self.0.get(&index)
    }

    pub fn insert(&mut self, index: usize, reg: Register) {
        self.0.insert(index, reg);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn keys(&self) -> Keys<usize, Register> {
        self.0.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &Register> {
        self.0.values()
    }

    pub fn to_string(&self) -> String {
        self.0
            .values()
            .map(|r| format!("{};", r.to_string_extended()))
            .collect()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Memory {
    pub min_address: u64,
    pub max_address: u64,
    pub last_address: u64,
    pub min_value: u64,
    pub max_value: u64,
    pub last_value: u64,
}

impl Memory {
    pub fn to_string(&self) -> String {
        format!(
            "memory: {:#018x};{:#018x};{:#018x};{:#018x};{:#018x};{:#018x}",
            self.min_address,
            self.max_address,
            self.last_address,
            self.min_value,
            self.max_address,
            self.last_value
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub address: usize,
    pub mnemonic: String,
    pub registers_min: Registers,
    pub registers_max: Registers,
    pub successors: Vec<Successor>,
}

impl Instruction {
    pub fn to_string(&self) -> String {
        let mut ret = String::new();
        ret.push_str(&format!("{:#018x};", self.address));
        ret.push_str(&format!("{};", self.mnemonic));

        for index in 0..REGISTERS.len() {
            if let Some(register) = self.registers_min.get(index) {
                ret.push_str(&format!(
                    "{}: {};",
                    REGISTERS[index],
                    register.to_string_extended()
                ));
            }
            if let Some(register) = self.registers_max.get(index) {
                ret.push_str(&format!(
                    "{}: {};",
                    REGISTERS[index],
                    register.to_string_extended()
                ));
            }
        }

        for successor in self.successors.iter() {
            ret.push_str(&format!("successor: {};", successor.to_string()));
        }

        ret
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializedInstruction {
    pub address: usize,
    pub mnemonic: String,
    pub registers_min: Registers,
    pub registers_max: Registers,
    pub registers_last: Registers,
    pub last_successor: usize,
    pub count: usize,
    pub memory: Option<Memory>,
}

impl SerializedInstruction {
    fn add_mem_to_registers(&self) -> (Registers, Registers, Registers) {
        let mut registers_min = self.registers_min.clone();
        let mut registers_max = self.registers_max.clone();
        let mut registers_last = self.registers_last.clone();

        if let Some(memory) = &self.memory {
            registers_min.insert(23, Register::new("memory_address", memory.min_address));
            registers_max.insert(23, Register::new("memory_address", memory.max_address));
            registers_last.insert(23, Register::new("memory_address", memory.last_address));

            registers_min.insert(24, Register::new("memory_value", memory.min_value));
            registers_max.insert(24, Register::new("memory_value", memory.max_value));
            registers_last.insert(24, Register::new("memory_value", memory.last_value));
        }

        (registers_min, registers_max, registers_last)
    }

    pub fn to_instruction(&self) -> Instruction {
        let (registers_min, registers_max, _) = self.add_mem_to_registers();

        Instruction {
            address: self.address,
            mnemonic: self.mnemonic.to_string(),
            registers_min,
            registers_max,
            successors: vec![],
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct SerializedEdge {
    from: usize,
    to: usize,
    count: usize,
}

#[derive(Clone, Serialize, Deserialize)]
struct SerializedTrace {
    pub instructions: Vec<SerializedInstruction>,
    pub edges: Vec<SerializedEdge>,
    pub first_address: usize,
    pub last_address: usize,
    pub image_base: usize,
}

impl SerializedTrace {
    pub fn to_trace(name: String, serialized: SerializedTrace) -> Trace {
        let mut instructions: HashMap<usize, Instruction> = serialized
            .instructions
            .into_iter()
            .map(|instr| (instr.address, instr.to_instruction()))
            .collect();
        for edge in &serialized.edges {
            if let Some(entry) = instructions.get_mut(&edge.from) {
                entry.successors.push(Successor { address: edge.to });
            }
        }
        for v in instructions.values_mut() {
            v.successors.sort_by(|a, b| a.address.cmp(&b.address))
        }

        Trace {
            name,
            instructions,
            image_base: serialized.image_base,
            first_address: serialized.first_address,
            last_address: serialized.last_address,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Copy)]
pub struct Successor {
    pub address: usize,
}

impl Successor {
    pub fn to_string(&self) -> String {
        format!("{:#018x}", self.address)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Trace {
    pub name: String,
    pub image_base: usize,
    pub instructions: HashMap<usize, Instruction>,
    pub first_address: usize,
    pub last_address: usize,
}

impl Trace {
    pub fn from_trace_file(file_path: String) -> Trace {
        let content =
            fs::read_to_string(&file_path).expect(&format!("File {} not found!", &file_path));
        Trace::from_file(file_path, content)
    }

    pub fn from_zip_file(file_path: String) -> Trace {
        let zip_file =
            fs::File::open(&file_path).expect(&format!("Could not open file {}", &file_path));
        let mut zip_archive = zip::ZipArchive::new(zip_file)
            .expect(&format!("Could not open archive {}", &file_path));

        let mut trace_file = zip_archive.by_index(0).unwrap();
        let trace_file_path = trace_file.sanitized_name().to_str().unwrap().to_string();

        let mut trace_content = String::new();
        trace_file
            .read_to_string(&mut trace_content)
            .expect(&format!("Could not read unzipped file {}", trace_file_path));

        Trace::from_file(trace_file_path, trace_content)
    }

    fn from_file(file_path: String, content: String) -> Trace {
        let serialized_trace: SerializedTrace = serde_json::from_str(&content)
            .expect(&format!("Could not deserialize file {}", &file_path));
        SerializedTrace::to_trace(file_path, serialized_trace)
    }

    pub fn visited_addresses(&self) -> HashSet<usize> {
        self.instructions.keys().map(|x| *x).collect()
    }

    pub fn to_string(&self) -> String {
        format!(
            "{};{:#018x};{:#018x};{:#018x}",
            self.name, self.image_base, self.first_address, self.last_address
        )
    }
}

pub struct TraceVec(pub Vec<Trace>);

impl TraceVec {
    pub fn from_vec(v: Vec<Trace>) -> TraceVec {
        TraceVec(v)
    }

    pub fn iter_instructions_at_address(
        &self,
        address: usize,
    ) -> impl Iterator<Item = &Instruction> {
        self.iter()
            .filter(move |t| t.instructions.contains_key(&address))
            .map(move |t| t.instructions.get(&address).unwrap())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter_all_instructions(&self) -> impl Iterator<Item = &Instruction> {
        self.0.iter().flat_map(|t| t.instructions.values())
    }

    pub fn iter(&self) -> impl Iterator<Item = &Trace> {
        self.0.iter()
    }

    pub fn as_slice(&self) -> &[Trace] {
        self.0.as_slice()
    }
}
