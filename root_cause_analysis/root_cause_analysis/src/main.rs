use root_cause_analysis::config::Config;
use root_cause_analysis::monitor::monitor_predicates;
use root_cause_analysis::rankings::rank_predicates;
use root_cause_analysis::traces::analyze_traces;
use std::time::Instant;
use structopt::StructOpt;

fn main() {
    let config = Config::from_args();

    let total_time = Instant::now();

    if config.analyze_traces() {
        println!("analyzing traces");
        let trace_analysis_time = Instant::now();
        analyze_traces(&config);
        println!(
            "trace analysis time: {} seconds",
            trace_analysis_time.elapsed().as_secs_f64()
        );
    }

    if config.monitor_predicates {
        println!("monitoring predicates");
        let monitoring_time = Instant::now();
        monitor_predicates(&config);
        println!(
            "monitoring time: {} seconds",
            monitoring_time.elapsed().as_secs_f64()
        );
    }

    if config.rank_predicates {
        println!("ranking predicates");
        let ranking_time = Instant::now();
        rank_predicates(&config);
        println!(
            "ranking time: {} seconds",
            ranking_time.elapsed().as_secs_f64()
        );
    }

    println!("total time: {} seconds", total_time.elapsed().as_secs_f64());
}
