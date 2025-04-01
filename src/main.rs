use std::io::BufReader;
use std::{io::BufRead, path::PathBuf};
use std::env;

use rustsat::solvers::Solve;
use rustsat::types::{Clause, Lit};
use rustsat::{instances::SatInstance, solvers::SolverResult, types::TernaryVal};
use rustsat_minisat::core::Minisat;
use structures::{clause_table::ClauseTable, node::SatSwarm};

mod structures;
// TODO: i think there is an issue with aborted resets

static mut GLOBAL_CLOCK: u64 = 0;
pub fn get_clock() -> &'static u64 {
    unsafe { &GLOBAL_CLOCK }
}

fn get_test_files() -> Option<Vec<std::path::PathBuf>> {
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

    collect_files(std::path::Path::new("/Users/tatestaples/Code/SatSwarm/src/tests"), &mut files);
    Some(files)
}

fn minisat_file(path: PathBuf) -> bool {
    let file = std::fs::File::open(path).expect("Unable to open file");
    let mut reader = BufReader::new(file);
    let instance: SatInstance = SatInstance::from_dimacs(&mut reader).unwrap();
    let mut solver: Minisat = rustsat_minisat::core::Minisat::default();
    let res = solver.solve().unwrap();
    solver.add_cnf(instance.into_cnf().0).unwrap();
    res == SolverResult::Sat
}
fn minisat_table(table: &ClauseTable) -> bool {
    let mut instance: SatInstance = SatInstance::new();
    for clause in table.clause_table.iter() {
        let clause: Clause = clause.iter().map(|&x| Lit::new(x.var as u32, x.negated)).collect();
        instance.add_clause(clause);
    }
    let mut solver: Minisat = rustsat_minisat::core::Minisat::default();
    let res = solver.solve().unwrap();
    solver.add_cnf(instance.into_cnf().0).unwrap();
    res == SolverResult::Sat
}


pub const DEBUG_PRINT: bool = false;

pub struct TestResult {
    pub simulated_result: bool,
    pub expected_result: bool,
    pub simulated_cycles: u64,
    pub config: TestConfig,
    pub testcase: String, 
    pub cycles_busy: u64,
    pub cycles_idle: u64,
}
pub struct TestConfig {
    pub num_nodes: i32,
    pub table_bandwidth: u8,
    pub topology: Topology,
    pub node_bandwidth: u8,
}
pub enum Topology {
    Grid,
    Torus,
    Dense,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut num_nodes = 1; // Default value for --num_nodes
    let mut topology = String::from("grid"); // Default value for --topology

    // Parse command-line arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--num_nodes" => {
                if i + 1 < args.len() {
                    num_nodes = args[i + 1].parse::<i32>().unwrap_or_else(|_| {
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
                    match value {
                        "grid" | "torus" | "dense"  => {
                            topology = value.to_string();
                        }
                        _ => {
                            eprintln!("Invalid value for --topology: {}", value);
                            eprintln!("Valid options are: grid, torus, dense");
                            std::process::exit(1);
                        }
                    }
                    i += 1; // Skip the value
                } else {
                    eprintln!("Missing value for --topology");
                    std::process::exit(1);
                }
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

    // load the first test file from ./tests (use OS to get the path)
    if let Some(files) = get_test_files() {
        for file in files.into_iter() {
            let f_copy = file.clone();
            let (clause_table, expected_result) = ClauseTable::load_file(file);
            // println!("Expected Result: {}, file: {:?}, num_terms: {}", expected_result, f_copy, clause_table.number_of_vars());
            // skip if the cluase table > 25 or expected result is unsat
            if clause_table.number_of_vars() > 20 || !expected_result {
                continue;
            }
            println!("Running test: {:?}", f_copy);
            let mut simulation = SatSwarm::grid(clause_table, 10, 10);
            let (sat, cycles) = simulation.test_satisfiability();
            println!("Satisfiable: {}({}), Cycles: {}", sat,expected_result, cycles);
        }
    } else {
        println!("No tests directory found");
    }
    println!("Done");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_satisfiability() {
        if let Some(files) = get_test_files() {
            for file in files.into_iter() {
                println!("Running test: {:?}", file);
                let (clause_table, expected_result) = ClauseTable::load_file(file);
                let mut simulation = SatSwarm::grid(clause_table, 10, 10);
                let (sat, cycles) = simulation.test_satisfiability();
                println!("Satisfiable: {}, Cycles: {}", sat, cycles);
                assert!(sat==expected_result, "Test failed");
            }
        } else {
            println!("No tests directory found");
        }
    }

    #[test]
    fn random_smalls() {
        for test in 0..10000 {
            let table = ClauseTable::random(10, 3);
            let mut simulation = SatSwarm::grid(table, 10, 10);
            let (sat, cycles) = simulation.test_satisfiability();
            if !sat {
                println!("Satisfiable: {}, Cycles: {}", sat, cycles);
            } else if test % 100 == 0 {
                println!("{}/10000", test);
            }

        }
    }
}
