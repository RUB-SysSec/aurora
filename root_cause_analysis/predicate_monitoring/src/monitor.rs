use log::debug;
use ptracer::Ptracer;
use std::env;
use std::fs;
use std::path::Path;
use trace_analysis::predicates::SerializedPredicate;

fn deserialize_predicates(predicate_file: &String) -> Vec<SerializedPredicate> {
    let content = fs::read_to_string(predicate_file).expect("Could not read predicates.json");

    serde_json::from_str(&content).expect("Could not deserialize predicates.")
}

fn serialize_ranking(out_file: &String, ranking: &Vec<usize>) {
    let content = serde_json::to_string(&ranking).expect("Could not serialize ranking");
    fs::write(out_file, content).expect(&format!("Could not write {}", out_file));
}

fn main() {
    match env::var("RUST_LOG") {
        Err(_) => {
            env::set_var("RUST_LOG", "error");
        }
        Ok(_) => {}
    }
    env_logger::init();

    let args: Vec<_> = env::args().collect();
    debug!("args = {:#?}", args);

    if args.len() < 4 {
        println!(
            "usage: {} <out file> <predicate file> <timeout> <command> [argument]...",
            args[0]
        );
        return;
    }

    let out_file = args.get(1).expect("No out file specified");
    let predicate_file = args.get(2).expect("No predicate file specified");
    let timeout: u64 = args
        .get(3)
        .expect("No timeout specified")
        .parse()
        .expect("Could not parse timeout");
    let cmd = args.get(4).expect("No cmd line specified");
    let cmd_args: Vec<_> = args[5..].iter().cloned().collect();

    debug!("cmd = {:?}", cmd);
    debug!("cmd_args = {:?}", cmd_args);

    let dbg = Ptracer::spawn(Path::new(&cmd), cmd_args.as_ref()).expect("spawn failed");

    let predicates = deserialize_predicates(&predicate_file);
    let ranking = predicate_monitoring::rank_predicates(dbg, predicates, timeout);

    serialize_ranking(out_file, &ranking);
}
