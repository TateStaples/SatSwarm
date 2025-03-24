use structures::{clause_table::ClauseTable, node::SatSwarm};

mod structures;

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

pub const DEBUG_PRINT: bool = false;

fn main() {
    // load the first test file from ./tests (use OS to get the path)
    if let Some(files) = get_test_files() {
        for file in files.into_iter() {
            let f_copy = file.clone();
            let (clause_table, expected_result) = ClauseTable::load_file(file);
            // println!("Expected Result: {}, file: {:?}, num_terms: {}", expected_result, f_copy, clause_table.number_of_vars());
            // skip if the cluase table > 25 or expected result is unsat
            if clause_table.number_of_vars() > 50 || expected_result {
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
}
