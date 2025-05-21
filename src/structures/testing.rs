#![allow(unused)]
use std::env;
use std::time::Duration;

use csv::Writer;
use std::fs::OpenOptions;
use crate::structures::clause_table::ClauseTable;
use crate::structures::minisat::minisat_table;
use crate::structures::network::Network;
use crate::structures::util_types::Time;

pub fn parse_topology(topology_str: &str, num_nodes: usize) -> Topology {
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


#[derive(Default, Clone, Debug)]
pub struct TestResult {
    pub simulated_result: bool,
    pub simulated_cycles: Time,
    pub cycles_busy: Time,
    pub cycles_idle: Time,
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
    pub num_vars: usize,
    pub test_dir: String,
    pub log_dir: String,
} impl TestConfig {
    pub fn new(num_nodes: usize, topology: Topology, node_bandwidth: usize, num_vars: usize, test_dir: String) -> Self {
        Self {
            num_nodes,
            topology: topology.clone(),
            node_bandwidth,
            num_vars,
            test_dir: test_dir.clone(),
            log_dir: TestConfig::config_name(num_nodes, topology, node_bandwidth, num_vars, test_dir)
        }
    }

    fn config_name(num_nodes: usize, topology: Topology, node_bandwidth: usize, num_vars: usize, test_dir: String) -> String {
        let test_name = test_dir.split('/').last().unwrap_or("unknown");
        let current_time = std::time::SystemTime::now();
        let current_time = current_time.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() % 1_000_000;
        let log_path = format!(
            "logs/{}-{:?}-{}-{}-{}_{}.csv",
            test_name, topology, num_nodes, node_bandwidth, num_vars, current_time
        );
        if std::path::Path::new(&log_path).exists() {
            eprintln!("Configuration with name '{}' already exists. Exiting to avoid overwriting logs.", log_path);
            std::process::exit(1);
        }
        log_path
    }
}


pub fn get_test_files(test_path: &str) -> Option<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    fn collect_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() {
                        // println!("Found test file: {:?}", path);
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
pub fn run_workload(test_path: String, config: TestConfig) {
    // load test files from the specified path
    if let Some(files) = get_test_files(&test_path) {
        for file in files.into_iter() {
            let f_copy = file.clone();
            let (mut clause_table, _) = ClauseTable::load_file(file);
            // skip if the clause table > 25 or expected result is unsat
            if clause_table.number_of_vars() != config.num_vars {
                continue;
            }
            println!("Running test: {:?}", f_copy);
            let (expected_result, minisat_speed) = minisat_table(&clause_table);
            let mut simulation = Network::generate(clause_table, &config);
            let start_time = std::time::Instant::now();
            let result = simulation.test_satisfiability();
            println!("Test took {} s", start_time.elapsed().as_secs_f64());
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

pub fn log_test(test_log: TestLog) {
    let log_file_path = &test_log.config.log_dir;

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

    println!("Logging test to: {}", log_file_path);
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
                    "Number of Variables"
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
                test_log.config.num_vars.to_string(),
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
