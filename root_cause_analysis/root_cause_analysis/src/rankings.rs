use crate::config::Config;
use crate::traces::{deserialize_mnemonics, deserialize_predicates};
use crate::utils::{read_file, write_file};
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use trace_analysis::predicates::SerializedPredicate;

pub fn trunc_score(score: f64) -> f64 {
    (score * 100.0).trunc() as f64
}
fn predicate_order(
    p1: &SerializedPredicate,
    p2: &SerializedPredicate,
    rankings: &Vec<Vec<usize>>,
) -> Ordering {
    p2.score.partial_cmp(&p1.score).unwrap().then(
        path_rank(p1.address, rankings)
            .partial_cmp(&path_rank(p2.address, rankings))
            .unwrap(),
    )
}

pub fn rank_predicates(config: &Config) {
    let rankings = deserialize_rankings(config);
    assert!(!rankings.is_empty(), f"No rankings in rankings.json");
    let mnemonics = deserialize_mnemonics(config);
    let mut predicates = deserialize_predicates(config);

    // Only sort if we have more than one predicate
    if predicates.len() > 1 {
        predicates.par_sort_by(|p1, p2| predicate_order(p1, p2, &rankings));
    }

    dump_ranked_predicates(config, &predicates, &mnemonics, &rankings);
}

fn path_rank(address: usize, rankings: &Vec<Vec<usize>>) -> f64 {
    rankings
        .par_iter()
        .map(|r| rank_path_level(address, r))
        .sum::<f64>()
        / rankings.len() as f64
}

fn rank_path_level(address: usize, rank: &Vec<usize>) -> f64 {
    match rank.iter().position(|x| address == *x) {
        Some(pos) => pos as f64 / rank.len() as f64,
        None => 2.0,
    }
}

pub fn serialize_rankings(config: &Config, rankings: &Vec<Vec<usize>>) {
    let content = serde_json::to_string(rankings).expect("Could not serialize rankings");
    write_file(&format!("{}/rankings.json", config.eval_dir), content);
}

fn deserialize_rankings(config: &Config) -> Vec<Vec<usize>> {
    let content = read_file(&format!("{}/rankings.json", config.eval_dir));
    serde_json::from_str(&content).expect("Could not deserialize rankings")
}

fn dump_ranked_predicates(
    config: &Config,
    predicates: &Vec<SerializedPredicate>,
    mnemonics: &HashMap<usize, String>,
    rankings: &Vec<Vec<usize>>,
) {
    let content: String = predicates
        .iter()
        .map(|p| {
            format!(
                "{} -- {} (path rank: {})\n",
                p.to_string(),
                mnemonics[&p.address],
                path_rank(p.address, rankings)
            )
        })
        .collect();
    write_file(
        &format!("{}/ranked_predicates.txt", config.eval_dir),
        content,
    );
}
