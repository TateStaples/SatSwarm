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

fn run_files() {
    if let Some(files) = get_test_files() {
        for file in files.into_iter() {
            let f_copy = file.clone();
            let (clause_table, expected_result) = ClauseTable::load_file(file);
            // println!("Expected Result: {}, file: {:?}, num_terms: {}", expected_result, f_copy, clause_table.number_of_vars());
            // skip if the cluase table > 25 or expected result is unsat
            if clause_table.num_vars > 50 || expected_result {
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
pub const DEBUG_PRINT: bool = false;

// TODO: implement multiple copies of the clause table so it doesn't bottleneck the networking
fn main() {
    run_random_tests();
}

fn run_random_tests() {
    for test in 0..100 {
        let table = ClauseTable::random(100, 20);
        let mut simulation = SatSwarm::grid(table, 10, 10);
        let (sat, cycles) = simulation.test_satisfiability();
        println!("Satisfiable: {}, Cycles: {}", sat, cycles);
        // println!("{}/10000", test);
    }
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

    #[test]
    fn random_mediums() {
        for test in 0..10000 {
            let table = ClauseTable::random(20, 8);
            let mut simulation = SatSwarm::grid(table, 10, 10);
            let (sat, cycles) = simulation.test_satisfiability();
            println!("Satisfiable: {}, Cycles: {}", sat, cycles);
            println!("{}/10000", test);

        }
    }
}
