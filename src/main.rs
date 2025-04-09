#![allow(unused)]
use std::env;
use std::time::Duration;

use csv::Writer;
use std::fs::OpenOptions;
use structures::minisat::minisat_table;
use structures::{clause_table::ClauseTable, satswarm::SatSwarm};

mod structures;

// example command: cargo run -- --num_nodes 64 --topology grid --test_path /Users/shaanyadav/Desktop/Projects/SatSwarm/src/tests
fn main() {
    // build_random_testset(51, 10, 3, 3);
    // return;
    let args: Vec<String> = env::args().collect();
    let mut num_nodes: usize = 1; // Default value for --num_nodes
    let mut topology = String::from("torus"); // Default value for --topology
    let mut test_path = String::from("tests"); // Default value for --test_path
    let mut node_bandwidth = 1_000_000; // Default value for --node_bandwidth
    let mut table_bandwidth = 1; // Default value for --table_bandwidth

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
            "--table_bandwidth" => {
                if i + 1 < args.len() {
                    table_bandwidth = args[i + 1].parse::<usize>().unwrap_or_else(|_| {
                        eprintln!("Invalid value for --table_bandwidth: {}", args[i + 1]);
                        std::process::exit(1);
                    });
                    i += 1; // Skip the value
                } else {
                    eprintln!("Missing value for --table_bandwidth");
                    std::process::exit(1);
                }
            }
            "--help" => {
                println!("Usage: cargo run -- [OPTIONS]");
                println!("Options:");
                println!("  --num_nodes <NUM>       Number of nodes (default: 100)");
                println!("  --topology <TOPOLOGY>   Topology (default: grid)");
                println!("  --test_path <PATH>      Path to test files (default: tests)");
                println!("  --node_bandwidth <BW>   Node bandwidth (default: 1_000_000)");
                println!("  --table_bandwidth <BW>  Table bandwidth (default: 1)");
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

    let config = TestConfig {
        num_nodes,
        topology: parse_topology(&topology, num_nodes),
        node_bandwidth,
        test_dir: test_path.clone(),
    };
    run_workload(test_path, config);

    println!("Done");
}

fn parse_topology(topology_str: &str, num_nodes: usize) -> Topology {
    match topology_str {
        "grid" => {
            let size = (num_nodes as f64).sqrt() as usize;
            Topology::Grid(size, size)
        }
        "torus" => {
            let size = (num_nodes as f64).sqrt() as usize;
            Topology::Torus(size, size)
        }
        "dense" => Topology::Dense(num_nodes as usize),
        _ => panic!("Invalid topology: {}", topology_str),
    }
}
#[derive(Debug, Clone)]
pub enum Topology {
    Grid(usize, usize),
    Torus(usize, usize),
    Dense(usize),
}


pub struct TestResult {
    pub simulated_result: bool,
    pub simulated_cycles: u64,
    pub cycles_busy: u64,
    pub cycles_idle: u64,
}
pub struct TestLog {
    pub test_result: TestResult,
    pub config: TestConfig,
    pub expected_result: bool,
    pub minisat_speed: Duration,
    pub test_path: String,
}
#[derive(Clone)]
pub struct TestConfig {
    pub num_nodes: usize,
    pub topology: Topology,
    pub node_bandwidth: usize,
    pub test_dir: String,
}


fn get_test_files(test_path: &str) -> Option<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    fn collect_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        let cnf = path.file_name().unwrap().to_str().unwrap().ends_with(".cnf");
                        if !cnf {
                            println!("Skipping file: {:?}", path);
                            continue;
                        }
                        // println!("Found test file: {:?} ends with .cnf? {}", path, path.file_name().unwrap().to_str().unwrap().ends_with(".cnf"));
                        files.push(path);
                    } else if path.is_dir() {
                        collect_files(&path, files);
                    }
                }
            }
        }
    }

    collect_files(std::path::Path::new(test_path), &mut files);
    Some(files)
}
fn run_workload(test_path: String, config: TestConfig) {
    // load test files from the specified path
    if let Some(files) = get_test_files(&test_path) {
        for file in files.into_iter() {
            let f_copy = file.clone();
            let (mut clause_table, _) = ClauseTable::load_file(file);
            // skip if the clause table > 25 or expected result is unsat
            if clause_table.number_of_vars() > 100 {
                continue;
            }
            println!("Running test: {:?}", f_copy);
            let (expected_result, minisat_speed) = minisat_table(&clause_table);
            let mut simulation = SatSwarm::generate(clause_table, &config);
            let result = simulation.test_satisfiability();
            assert!(result.simulated_result == expected_result, "Test failed: expected {}, got {}", expected_result, result.simulated_result);
            let test_log = TestLog {
                test_result: result,
                config: config.clone(),
                expected_result,
                minisat_speed,
                test_path: f_copy.to_str().unwrap_or("unknown").to_string(),
            };
            log_test(test_log);
        }
    } else {
        println!("No tests directory found at: {}", test_path);
    }
}
fn config_name(config: &TestConfig) -> String {
    let test_name = config.test_dir.split('/').last().unwrap_or("unknown");
    format!(
        "{}-{:?}-{}-{}",
        test_name, config.topology, config.num_nodes, config.node_bandwidth, 
    )
}
fn log_test(test_log: TestLog) {
    let log_file_path = format!("logs/{}.csv", config_name(&test_log.config));

    // Create logs directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all("logs") {
        eprintln!("Failed to create logs directory: {}", e);
        return;
    }

    // Open or create the CSV file
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path.clone());

    match file {
        Ok(file) => {
            let file_is_empty = file.metadata().map(|m| m.len() == 0).unwrap_or(false);
            let mut writer = Writer::from_writer(file);

            // Write the header if the file is empty
            if file_is_empty {
                if let Err(e) = writer.write_record(&[
                    "Test Path",
                    "Expected Result",
                    "Minisat Speed (ns)",
                    "Simulated Result",
                    "Simulated Cycles",
                    "Cycles Busy",
                    "Cycles Idle",
                    "Num Nodes",
                    "Topology",
                    "Node Bandwidth",
                ]) {
                    eprintln!("Failed to write CSV header: {}", e);
                    return;
                }
            }

            // Write the test log as a CSV record
            if let Err(e) = writer.write_record(&[
                test_log.test_path,
                test_log.expected_result.to_string(),
                test_log.minisat_speed.as_nanos().to_string(),
                test_log.test_result.simulated_result.to_string(),
                test_log.test_result.simulated_cycles.to_string(),
                test_log.test_result.cycles_busy.to_string(),
                test_log.test_result.cycles_idle.to_string(),
                test_log.config.num_nodes.to_string(),
                format!("{:?}", test_log.config.topology),
                test_log.config.node_bandwidth.to_string(),
            ]) {
                eprintln!("Failed to write CSV record: {}", e);
            }

            if let Err(e) = writer.flush() {
                eprintln!("Failed to flush CSV writer: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Failed to open log file: {}: {}", log_file_path, e);
        }
    }
}
