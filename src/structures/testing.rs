#![allow(unused)]
use std::env;
use std::time::Duration;

use csv::Writer;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use crate::structures::clause_table::ClauseTable;
use crate::structures::microsat;
use crate::structures::microsat::{ClauseId, Variable};
use crate::structures::minisat::minisat_table;
use crate::structures::network::Network;
use crate::structures::util_types::Time;

pub struct ProblemDescription {
    pub num_vars: usize,
    pub num_clauses: usize,
    pub test_name: String,
} 
impl ProblemDescription {
    pub fn from_path(path: PathBuf) -> Self {
        let num_clauses: usize = 0;
        let num_vars: usize = 0;
        println!("Loading problem from: {:?}", path);
        let file: String = std::fs::read_to_string(&path).unwrap();
        for line in file.lines() {
            // If the line starts with 'p', then it is a problem line
            if line.starts_with('p') {
                let mut parts = line.split_whitespace();
                let _ = parts.next(); // Skip the 'p'
                let _ = parts.next(); // Skip the 'cnf'
                let num_vars: Variable = parts.next().unwrap().parse().unwrap();
                let num_clauses: ClauseId = parts.next().unwrap().parse().unwrap();
                break;
            }
        }
        debug_assert_ne!(num_clauses, 0);
        debug_assert_ne!(num_vars, 0);
        Self {
            num_vars,
            num_clauses,
            test_name: path.file_name().unwrap().to_str().unwrap().to_string()
        }
    }
}
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
        "dense" => Topology::Dense(num_nodes),
        _ => panic!("Invalid topology: {}", topology_str),
    }
}
#[derive(Debug, Clone)]
pub enum Topology {
    Grid(usize, usize),
    Torus(usize, usize),
    Dense(usize),
}

#[derive(Clone)]
pub struct ArchitectureDescription {
    /// The arrangement of the nodes
    pub topology: Topology,
    /// How long it takes to choose a variable (typically 0)
    pub decision_delay: Time,
    /// How long it takes to send a fork between nodes
    pub fork_delay: Time,
    /// How many clauses are evaluated in parallel
    pub clause_per_eval: usize,
    /// How many cycles per clause evaluation
    pub cycles_per_eval: Time,
}
#[derive(Default, Clone, Debug)]
pub struct TestResult {
    pub simulated_result: bool,
    pub simulated_cycles: Time,
    pub cycles_busy: Time,
    pub cycles_idle: Time,
}
pub struct TestLog {
    pub problem_description: ProblemDescription,
    pub config: ArchitectureDescription,
    pub test_result: TestResult,
    pub expected_result: bool,
    pub minisat_speed: Duration,
}
impl TestLog {
    pub fn create_log_path() -> PathBuf {
        let now = chrono::Local::now();
        let log_path = format!(
            "logs/{}.csv",
            now.format("%Y-%m-%d,%H:%M")
        );
        if Path::new(&log_path).exists() {
            eprintln!("Configuration with name '{}' already exists. Exiting to avoid overwriting logs.", log_path);
            std::process::exit(1);
        }
        PathBuf::from(log_path)
    }
    /// Take the results from a simulation and log them to a CSV filed
    pub fn save(&self, log_file_path: PathBuf) {

        // Create logs directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all("logs") {
            eprintln!("Failed to create logs directory: {}", e);
            return;
        }

        println!("Logging test to: {:?}", log_file_path);
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
                        // Problem Description
                        "Test Path",            // Which problem
                        "Number of Variables",  // How many variables in the problem
                        "Number of Clauses",    // How many clauses in problem
                        // Solver Config
                        "Topology",             // Arangement of nodes
                        "Decision Delay",
                        "Fork Delay",
                        "Clause Per Eval",
                        "Cycles Per Eval",
                        // Solver Results
                        "Simulated Result",     // Result from our solver
                        "Simulated Cycles",     // # of cycles our solver took
                        "Cycles Busy",          // # of cycles x nodes that are processing
                        "Cycles Idle",          // # of cycles x nodes awaiting fork
                        // Usefult Comparisons
                        "Expected Result",      // Expected result
                        "Minisat Speed (ns)",   // Minisat speed
                    ]) {
                        eprintln!("Failed to write CSV header: {}", e);
                        return;
                    }
                }

                // Write the test log as a CSV record
                if let Err(e) = writer.write_record(&[
                    // Problem Description
                    self.problem_description.test_name.clone(),
                    self.problem_description.num_vars.to_string(),
                    self.problem_description.num_clauses.to_string(),
                    // Solver Config
                    format!("{:?}", self.config.topology),
                    self.config.decision_delay.to_string(),
                    self.config.fork_delay.to_string(),
                    self.config.clause_per_eval.to_string(),
                    self.config.cycles_per_eval.to_string(),
                    // Solver Results 
                    self.test_result.simulated_result.to_string(),
                    self.test_result.simulated_cycles.to_string(),
                    self.test_result.cycles_busy.to_string(),
                    self.test_result.cycles_idle.to_string(),
                    // Useful Comparisons
                    self.expected_result.to_string(),
                    self.minisat_speed.as_nanos().to_string(),
                ]) {
                    eprintln!("Failed to write CSV record: {}", e);
                }

                if let Err(e) = writer.flush() {
                    eprintln!("Failed to flush CSV writer: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to open log file: {:?}: {}", log_file_path, e);
            }
        }
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

pub fn gen_traces(test_path: String, variables: i32) {
    if let Some(files) = get_test_files(&test_path) {
        for file in files {
            let name = file.file_name().unwrap().to_str().unwrap().to_string();
            let pattern = format!("uf{}", variables);
            if name.contains(pattern.as_str()) || true {
                microsat::build_trace_path(file)
            }
        }
    } else { 
        println!("No test files found.");
    }
}
/// Entry point for running a SAT problem in the simulator
pub fn run_workload(test_path: String, config: ArchitectureDescription, var_filter: Option<usize>) {
    // load test files from the specified path
    let log_path = TestLog::create_log_path();
    if let Some(files) = get_test_files(&test_path) {
        for file in files.into_iter() {
            let (mut clause_table, _) = ClauseTable::load_file(file.clone());
            // skip if the clause table > 25 or expected result is unsat
            if var_filter.is_some() && clause_table.number_of_vars() != var_filter.unwrap() {
                println!("Clause table number of vars ({}) does not match config num vars ({})", clause_table.number_of_vars(), var_filter.unwrap());
                continue;
            }
            let problem_description = ProblemDescription::from_path(file.clone());
            println!("Running test: {:?}", file);
            let (expected_result, minisat_speed) = minisat_table(&clause_table);
            let mut simulation = Network::generate(clause_table, &config);
            let start_time = std::time::Instant::now();
            let result = simulation.test_satisfiability();
            println!("Test took {} s", start_time.elapsed().as_secs_f64());
            assert_eq!(result.simulated_result, expected_result, "Test failed: expected {}, got {}", expected_result, result.simulated_result);
            let test_log = TestLog {
                problem_description,
                test_result: result,
                config: config.clone(),
                expected_result,
                minisat_speed,
            };
            test_log.save(log_path.clone());
        }
    } else {
        println!("No tests directory found at: {}", test_path);
    }
}