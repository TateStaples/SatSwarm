use structures::{clause_table::ClauseTable, node::SatSwarm};

mod structures;

static mut GLOBAL_CLOCK: u64 = 0;
pub fn get_clock() -> &'static u64 {
    unsafe { &GLOBAL_CLOCK }
}

fn get_test_files() -> Option<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir("/Users/tatestaples/Code/SatSwarm/src/tests").ok()? {
        let entry = entry.ok()?;
        let  path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }
    Some(files)
}

pub const DEBUG_PRINT: bool = false;

fn main() {
    // load the first test file from ./tests (use OS to get the path)
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
