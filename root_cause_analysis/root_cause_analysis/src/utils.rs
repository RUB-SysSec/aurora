use glob::glob;
use std::fs;
use std::num::ParseIntError;

pub fn read_file(file_path: &str) -> String {
    fs::read_to_string(file_path).expect(&format!("Could not read file {}", file_path))
}

pub fn read_file_to_bytes(file_path: &str) -> Vec<u8> {
    fs::read(file_path).expect(&format!("Could not read file {}", file_path))
}

pub fn write_file(file_path: &str, content: String) {
    fs::write(file_path, content).expect(&format!("Could not write file {}", file_path));
}

pub fn glob_paths(pattern: String) -> Vec<String> {
    glob(&pattern)
        .unwrap()
        .map(|p| p.unwrap().to_str().unwrap().to_string())
        .collect()
}

pub fn parse_hex(src: &str) -> Result<usize, ParseIntError> {
    usize::from_str_radix(&src.replace("0x", ""), 16)
}
