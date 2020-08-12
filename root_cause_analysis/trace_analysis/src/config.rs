use std::num::ParseIntError;
use structopt::clap::AppSettings;
use structopt::StructOpt;

fn parse_hex(src: &str) -> Result<usize, ParseIntError> {
    usize::from_str_radix(&src.replace("0x", ""), 16)
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "trace_analysis",
    global_settings = &[AppSettings::DisableVersion]
)]

pub struct Config {
    #[structopt(index = 1, help = "Path to traces of crashing inputs")]
    pub path_to_crashes: String,
    #[structopt(index = 2, help = "Path to traces of non-crashing inputs")]
    pub path_to_non_crashes: String,
    #[structopt(
        short = "c",
        long = "check-traces",
        help = "Performs trace integrity checks"
    )]
    pub check_traces: bool,
    #[structopt(short = "d", long = "dump-traces", help = "Dumps trace data")]
    pub dump_traces: bool,
    #[structopt(short = "s", long = "scores", help = "Dumps instruction scores")]
    pub dump_scores: bool,
    #[structopt(long = "zip", help = "Trace files are provided in zipped form")]
    pub zipped: bool,
    #[structopt(short = "a", long = "dump-address", default_value="0", parse(try_from_str = parse_hex), help = "Dump at address")]
    pub dump_address: usize,
    #[structopt(
        short = "r",
        long = "random",
        default_value = "0",
        help = "Select n random traces"
    )]
    pub random_traces: usize,
    #[structopt(
        short = "f",
        long = "filter",
        help = "Ignore non-crashes that do not visit the crashing CFG leaves"
    )]
    pub filter_non_crashes: bool,
    #[structopt(short = "t", long = "trace-info", help = "Dump trace infos")]
    pub trace_info: bool,
    #[structopt(
        long = "output-dir",
        default_value = "./",
        help = "Path for output directory"
    )]
    pub output_directory: String,
    #[structopt(
        long = "blacklist-crashes",
        default_value = "",
        help = "Path for crash blacklist"
    )]
    pub crash_blacklist_path: String,
    #[structopt(
    long = "debug-predicate",
    default_value="0", parse(try_from_str = parse_hex),
    help = "Dumps the best predicate at address"
    )]
    pub predicate_address: usize,
}

impl Config {
    pub fn default(
        trace_dir: &String,
        output_dir: &Option<String>,
        crash_blacklist_path: &Option<String>,
    ) -> Config {
        Config {
            path_to_crashes: format!("{}/traces/crashes/", trace_dir),
            path_to_non_crashes: format!("{}/traces/non_crashes/", trace_dir),
            check_traces: false,
            dump_traces: false,
            dump_scores: true,
            zipped: true,
            dump_address: 0,
            random_traces: 0,
            filter_non_crashes: false,
            trace_info: false,
            output_directory: if output_dir.is_some() {
                output_dir.as_ref().unwrap().to_string()
            } else {
                "./".to_string()
            },
            crash_blacklist_path: if crash_blacklist_path.is_some() {
                crash_blacklist_path.as_ref().unwrap().to_string()
            } else {
                "".to_string()
            },
            predicate_address: 0,
        }
    }

    pub fn random_traces(&self) -> bool {
        self.random_traces > 0
    }

    pub fn dump_address(&self) -> bool {
        self.dump_address > 0
    }

    pub fn blacklist_crashes(&self) -> bool {
        self.crash_blacklist_path != ""
    }

    pub fn debug_predicate(&self) -> bool {
        self.predicate_address > 0
    }
}
