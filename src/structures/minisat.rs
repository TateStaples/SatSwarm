
use std::io::BufReader;
use std::{path::PathBuf};

use rustsat::solvers::Solve;
use rustsat::types::{Clause, Lit};
use rustsat::{instances::SatInstance, solvers::SolverResult};
use rustsat_minisat::core::Minisat;
use super::{clause_table::ClauseTable, node::SatSwarm};

pub fn minisat_file(path: PathBuf) -> bool {
    let file = std::fs::File::open(path).expect("Unable to open file");
    let mut reader = BufReader::new(file);
    let instance: SatInstance = SatInstance::from_dimacs(&mut reader).unwrap();
    let mut solver: Minisat = rustsat_minisat::core::Minisat::default();
    let res = solver.solve().unwrap();
    solver.add_cnf(instance.into_cnf().0).unwrap();
    res == SolverResult::Sat
}
pub fn minisat_table(table: &ClauseTable) -> bool {
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

pub fn build_random_testset(clauses: usize, vars: u8, sats: usize, unsats: usize) {
    let mut sats_made = 0;
    let mut unsats_made = 0;
    // next file is the next file of the form tests/sat/random/{clauses}_{vars}_i.cnf
    while sats_made < sats || unsats_made < unsats {
        let mut table = ClauseTable::random(clauses, vars);
        
        if minisat_table(&table) {
            if sats_made < sats {
                let file_path = format!("tests/random/sat/{}_{}_{}.cnf", clauses, vars, sats_made);
                println!("Sat file path: {}", file_path);
                let pathbuf = PathBuf::from(file_path);
                let _ = table.write_file(pathbuf);
                sats_made += 1;
            }
        } else {
            if unsats_made < unsats {
                let file_path = format!("tests/random/sat/{}_{}_{}.cnf", clauses, vars, unsats_made);
                println!("Unsat file path: {}", file_path);
                let pathbuf = PathBuf::from(file_path);
                let _ = table.write_file(pathbuf);
                unsats_made += 1;
            }
        }
    }
    
}