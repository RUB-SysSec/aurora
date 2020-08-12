use structopt::StructOpt;
use trace_analysis::config::Config;
use trace_analysis::debug::{
    debug_predicate_at_address, diff_traces, diff_traces_at_address, dump_trace_info,
};
use trace_analysis::trace_analyzer::TraceAnalyzer;

fn main() {
    let config = Config::from_args();

    let trace_analyzer = TraceAnalyzer::new(&config);

    if config.dump_traces {
        println!("dumping traces");
        diff_traces(&config, &trace_analyzer);
    }

    if config.dump_address() {
        println!("dumping traces at address 0x{:x}", config.dump_address);
        diff_traces_at_address(&config, &trace_analyzer);
    }

    if config.trace_info {
        println!("dumping trace information");
        dump_trace_info(&config, &trace_analyzer);
    }

    if config.debug_predicate() {
        println!(
            "dumping predicate at address 0x{:x}",
            config.predicate_address
        );
        debug_predicate_at_address(config.predicate_address, &trace_analyzer);
    }

    if config.dump_scores {
        println!("dumping linear scores");
        trace_analyzer.dump_scores(&config, false, false);
    }
}
