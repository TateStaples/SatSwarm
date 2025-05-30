#![allow(unused)]
use std::env;
use std::time::Duration;

use csv::Writer;
use std::fs::OpenOptions;
use std::process::exit;
use structures::minisat::minisat_table;
use structures::{clause_table::ClauseTable, network::Network};
use crate::structures::{microsat, testing};
use crate::structures::testing::{parse_topology, run_workload, TestConfig};

mod structures;

// example command: cargo run -- --num_nodes 64 --topology grid --test_path /Users/shaanyadav/Desktop/Projects/SatSwarm/src/tests --node_bandwidth 100 --num_vars 50
fn main() {
    // microsat::main();
    // exit(1);
    
    let args: Vec<String> = env::args().collect();
    let mut num_nodes: usize = 256; // Default value for --num_nodes
    let mut topology = String::from("torus"); // Default value for --topology
    let mut test_path = String::from("tests/satlib/unsat"); // Default value for --test_path
    let mut node_bandwidth = 100; // Default value for --node_bandwidth
    let mut num_vars = 200; // Default value for --num_vars

    testing::gen_traces(test_path.clone(), 125);
    exit(1);

    // Parse command-line arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--num_nodes" => {
                if i + 1 < args.len() {
                    num_nodes = args[i + 1].parse::<usize>().unwrap_or_else(|_| {
                        eprintln!("Invalid value for --num_nodes: {}", args[i + 1]);
                        std::process::exit(1);
                    });
                    i += 1; // Skip the value
                } else {
                    eprintln!("Missing value for --num_nodes");
                    std::process::exit(1);
                }
            }
            "--topology" => {
                if i + 1 < args.len() {
                    let value = args[i + 1].as_str();
                    topology = value.to_string();
                    i += 1; // Skip the value
                } else {
                    eprintln!("Missing value for --topology");
                    std::process::exit(1);
                }
            }
            "--test_path" => {
                if i + 1 < args.len() {
                    test_path = args[i + 1].clone();
                    i += 1; // Skip the value
                } else {
                    eprintln!("Missing value for --test_path");
                    std::process::exit(1);
                }
            }
            "--node_bandwidth" => {
                if i + 1 < args.len() {
                    node_bandwidth = args[i + 1].parse::<usize>().unwrap_or_else(|_| {
                        eprintln!("Invalid value for --node_bandwidth: {}", args[i + 1]);
                        std::process::exit(1);
                    });
                    i += 1; // Skip the value
                } else {
                    eprintln!("Missing value for --node_bandwidth");
                    std::process::exit(1);
                }
            }
            "--num_vars" => {
                if i + 1 < args.len() {
                    num_vars = args[i + 1].parse::<usize>().unwrap_or_else(|_| {
                        eprintln!("Invalid value for --num_vars: {}", args[i + 1]);
                        std::process::exit(1);
                    });
                    i += 1; // Skip the value
                } else {
                    eprintln!("Missing value for --num_vars");
                    std::process::exit(1);
                }
            }
            "--help" => {
                println!("Usage: cargo run -- [OPTIONS]");
                println!("Options:");
                println!("  --num_nodes <NUM>       Number of nodes (default: 100)");
                println!("  --topology <TOPOLOGY>   Topology (default: grid)");
                println!("  --test_path <PATH>      Path to test files (default: tests)");
                println!("  --node_bandwidth <BW>   Node bandwidth (default: 100)");
                println!("  --num_vars <NUM>        Number of variables (default: 50)");
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    println!("Number of nodes: {}", num_nodes);
    println!("Topology: {}", topology);
    println!("Test path: {}", test_path);

    let config = TestConfig::new(
        num_nodes,
        parse_topology(&topology, num_nodes),
        node_bandwidth,
        num_vars,
        test_path.clone(),
    );
    run_workload(test_path, config);

    println!("Done");
}

// #[cfg(test)]
// mod tests {
//     use microsat::dimacs_parser::parse_dimacs;
//     use microsat::solver::solve;
// 
//     #[test]
//     fn microsat_test() {
//         println!("the very beginning");
//         let path = "/Users/tatestaples/Code/SatSwarm/tests/satlib/sat/uf20-0100.cnf";
//         let expression = parse_dimacs(path);
//         println!("Expression {:?}", expression.num_active_clauses);
//         println!("starting");
//         let start_time = std::time::Instant::now();
//         let result = solve(expression, false, false);
//         println!("Time: {}", start_time.elapsed().as_secs_f64());
//         assert!(result.is_none());
//     }
// }