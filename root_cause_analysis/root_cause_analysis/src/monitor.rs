use crate::config::Config;
use crate::rankings::serialize_rankings;
use crate::utils::{glob_paths, read_file};
use rayon::prelude::*;
use std::fs::File;
use std::fs::{read_to_string, remove_file};
use std::process::{Child, Command, Stdio};
use std::time::Instant;
use trace_analysis::trace_analyzer::{blacklist_path, read_crash_blacklist};

pub fn monitor_predicates(config: &Config) {
    let cmd_line = cmd_line(&config);
    let blacklist_paths =
        read_crash_blacklist(config.blacklist_crashes(), &config.crash_blacklist_path);

    let rankings = glob_paths(format!("{}/inputs/crashes/*", config.eval_dir))
        .into_par_iter()
        .enumerate()
        .filter(|(_, p)| !blacklist_path(&p, &blacklist_paths))
        .map(|(index, i)| monitor(config, index, &replace_input(&cmd_line, &i)))
        .filter(|r| !r.is_empty())
        .collect();

    serialize_rankings(config, &rankings);
}

pub fn monitor(
    config: &Config,
    index: usize,
    (cmd_line, file_path): &(String, Option<String>),
) -> Vec<usize> {
    let predicate_order_file = format!("out_{}", index);
    let predicate_file = &format!("{}/{}", config.eval_dir, predicate_file_name());
    let timeout = format!("{}", config.monitor_timeout);

    let args: Vec<_> = cmd_line.split_whitespace().map(|s| s.to_string()).collect();

    let mut child = if let Some(p) = file_path {
        Command::new("./target/release/monitor")
            .arg(&predicate_order_file)
            .arg(&predicate_file)
            .arg(&timeout)
            .args(args)
            .stdin(Stdio::from(File::open(p).unwrap()))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Could not spawn child")
    } else {
        Command::new("./target/release/monitor")
            .arg(&predicate_order_file)
            .arg(&predicate_file)
            .arg(&timeout)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Could not spawn child")
    };

    wait_and_kill_child(&mut child, config.monitor_timeout);

    deserialize_predicate_order_file(&predicate_order_file)
}

fn wait_and_kill_child(child: &mut Child, timeout: u64) {
    let start_time = Instant::now();

    while start_time.elapsed().as_secs() < timeout + 10 {
        match child.try_wait() {
            Ok(Some(_)) => break,
            _ => {}
        }
    }

    match child.kill() {
        _ => {}
    }
}

fn predicate_file_name() -> String {
    "predicates.json".to_string()
}

fn deserialize_predicate_order_file(file_path: &String) -> Vec<usize> {
    let content = read_to_string(file_path);

    if !content.is_ok() {
        return vec![];
    }

    let ret: Vec<usize> = serde_json::from_str(&content.unwrap())
        .expect(&format!("Could not deserialize {}", file_path));
    remove_file(file_path).expect(&format!("Could not remove {}", file_path));

    ret
}

pub fn cmd_line(config: &Config) -> String {
    let executable = executable(config);
    let arguments = parse_args(config);

    format!("{} {}", executable, arguments)
}

fn parse_args(config: &Config) -> String {
    let file_name = format!("{}/arguments.txt", config.eval_dir);
    read_file(&file_name)
}

pub fn executable(config: &Config) -> String {
    let pattern = format!("{}/*_trace", config.eval_dir);
    let mut results = glob_paths(pattern);
    assert_eq!(results.len(), 1);

    results.pop().expect("No trace executable found")
}

pub fn replace_input(cmd_line: &String, replacement: &String) -> (String, Option<String>) {
    match cmd_line.contains("@@") {
        true => (cmd_line.replace("@@", replacement), None),
        false => (cmd_line.to_string(), Some(replacement.to_string())),
    }
}
