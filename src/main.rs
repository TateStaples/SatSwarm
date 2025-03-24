use structures::{clause_table::ClauseTable, node::SatSwarm};

mod structures;

static mut GLOBAL_CLOCK: u64 = 0;
pub fn get_clock() -> &'static u64 {
    unsafe{&GLOBAL_CLOCK}
}

fn get_test_files() -> Option<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir("/Users/tatestaples/Code/SatSwarm/src/tests").ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }
    Some(files)
}

fn main() {
    // load the first test file from ./tests (use OS to get the path)
    if let Some(files) = get_test_files() {
        if let Some(file) = files.into_iter().next() {
            let clause_table = ClauseTable::load_file(file);
            let mut simulation = SatSwarm::grid(clause_table, 10, 10);
            let sat = simulation.test_satisfiability();
            println!("Satisfiable: {}", sat);
        } else {
            println!("No files found in ./tests");
        }
    } else {
        println!("No tests directory found");
    }
}
