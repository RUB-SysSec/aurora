use std::num::ParseIntError;
use structopt::clap::AppSettings;
use structopt::StructOpt;

fn parse_hex(src: &str) -> Result<usize, ParseIntError> {
    usize::from_str_radix(&src.replace("0x", ""), 16)
}

#[derive(Debug, StructOpt)]
#[structopt(
name = "root_cause_analysis",
global_settings = &[AppSettings::DisableVersion]
)]

pub struct Config {
    #[structopt(long = "trace-dir", default_value = "", help = "Path to traces")]
    pub trace_dir: String,
    #[structopt(long = "eval-dir", help = "Path to evaluation folder")]
    pub eval_dir: String,
    #[structopt(long = "rank-predicates", help = "Rank predicates")]
    pub rank_predicates: bool,
    #[structopt(long = "monitor", help = "Monitor predicates")]
    pub monitor_predicates: bool,
    #[structopt(
        long = "--monitor-timeout",
        default_value = "60",
        help = "Timeout for monitoring"
    )]
    pub monitor_timeout: u64,
    #[structopt(
        long = "blacklist-crashes",
        default_value = "",
        help = "Path for crash blacklist"
    )]
    pub crash_blacklist_path: String,
    #[structopt(long = "debug-trace", help = "Debug trace")]
    pub debug_trace: bool,
    #[structopt(
        long = "load-offset",
        default_value = "0x0000555555554000",
        parse(try_from_str = parse_hex),
        help = "Load offset of the target"
    )]
    pub load_offset: usize,
}

impl Config {
    pub fn analyze_traces(&self) -> bool {
        !self.trace_dir.is_empty()
    }

    pub fn monitor_predicates(&self) -> bool {
        !self.eval_dir.is_empty()
    }

    pub fn blacklist_crashes(&self) -> bool {
        self.crash_blacklist_path != ""
    }
}
