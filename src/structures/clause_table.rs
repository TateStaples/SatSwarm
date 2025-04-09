use std::{fs::File, io::Write as IoWrite};
use std::{io::BufRead, path::PathBuf};
use super::util_types::{NodeId, VarId, CLAUSE_LENGTH}; 
struct Query {
    source: NodeId,
    var: VarId,
    set: bool,
    reset: bool,
    updates_left: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Term {
    pub var: VarId,
    pub negated: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermState {False, True, Symbolic} // True is not needed since the clause is satisfied when any term is true
impl Default for TermState {fn default() -> Self {TermState::Symbolic}}
pub type ClauseState = [TermState; CLAUSE_LENGTH];
pub type CNFState = Vec<ClauseState>;
pub struct ClauseTable {
    pub clause_table: Vec<[(Term, TermState); CLAUSE_LENGTH]>,   // 2D Vec to store the table of clauses
    pub num_clauses: usize,           // Number of clauses in the table
    pub num_vars: usize,              // Number of variables in the table
}

impl ClauseTable {
    pub fn _dummy() -> Self {
        let num_clauses = 10; // Number of clauses in the table
        Self {
            clause_table: vec![[Default::default(); CLAUSE_LENGTH]; num_clauses as usize], // Initialize the clause table with 0s
            num_clauses: num_clauses, // Initialize the number of clauses
            num_vars: 1,
        }
    }

    pub fn random(num_clauses: usize, num_vars: u8) -> Self {
        let mut clause_table = Vec::with_capacity(num_clauses);
        for _ in 0..num_clauses {
            let mut clause = [(Term{var: 0, negated: false}, TermState::Symbolic); CLAUSE_LENGTH];
            for i in 0..CLAUSE_LENGTH {
                let var = ((rand::random::<u8>() % num_vars) + 1) as u8;
                let negated = rand::random::<bool>();
                clause[i] = (Term{var, negated}, TermState::Symbolic);
            }
            clause_table.push(clause);
        }
        clause_table.push([(Term{var: 0, negated: true}, TermState::Symbolic); CLAUSE_LENGTH]);  // Add a dummy clause to the end to make var 0 false
        // clause_table.push([Term{var: 0, negated: false}; CLAUSE_LENGTH]);  // Add a dummy clause to the end to make var 0 true (contradiction)
        let num_clauses = clause_table.len();
        Self {
            clause_table,
            num_clauses,
            num_vars: (num_vars as usize),
        }
    }

    pub fn load_file(file: PathBuf) -> (Self, bool) {
        // Load a file and return a new ClauseTable with expected SAT result
        /* Example File Format                                  (0 is the end of the clause)
        c
        c SAT instance in DIMACS CNF input format.
        c
        p cnf 100 286                                           p cnf <num_vars> <num_clauses>
        80  -39  -21  0                                         <var1> <var2> ... <varN> 0                   
        -58  25  23  0
        -88  55  -42  0
        -71  -49  46  0
         */
        let mut num_clauses = 0;
        let sat = !file.to_string_lossy().to_lowercase().contains("unsat");
        let mut clauses = Vec::new();
        let mut var_count = 0;
        let file = std::fs::File::open(file).unwrap();
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap();
            // println!("{}", line);
            let mut clause = [Default::default(); CLAUSE_LENGTH];
            let mut clause_end = false;
            if line.starts_with("p cnf") {  // Parse the number of variables and clauses *header*
                let mut parts = line.split_whitespace();
                // println!("{:?}", parts.clone().collect::<Vec<&str>>());
                parts.next(); // Skip "p"
                parts.next(); // Skip "cnf"
                var_count = parts.next().unwrap().parse().unwrap();
                assert!(var_count < u8::MAX as i32, "Too many variables for u8");
                num_clauses = parts.next().unwrap().parse().unwrap();
                clauses = Vec::with_capacity(num_clauses);
            } else if line.starts_with("c") {  // Skip comments
                continue;
            } else if line.starts_with("%") {  // end this file
                break;
            } else {
                let parts = line.split_whitespace();
                for (term_index, part) in parts.enumerate() {
                    assert!(!clause_end, "Clause has already ended");
                    let num: i32 = part.parse().unwrap();
                    if num == 0 {
                        clause_end = true;
                        assert!(term_index <= CLAUSE_LENGTH, "Only 3SAT is supported");
                        for i in term_index..CLAUSE_LENGTH {
                            clause[i] = (Term{var: 0, negated: false}, TermState::Symbolic);  // Var 0 is always false
                        }
                    } else {
                        assert!(num.abs() < u8::MAX as i32, "Too many variables for u8");
                        clause[term_index] = (Term{var: num.abs() as u8, negated: num < 0}, TermState::Symbolic);  // want to 0 index the variables
                    }
                }
            }
            if clause_end {
                clauses.push(clause);
            }
        }
        if num_clauses < 10 {
            println!("Clauses: {:?}, expected_num_clauses: {}, expected_sat: {}, expected_vars: {}", clauses, num_clauses, sat, var_count);
        }
        assert!(clauses.len() == num_clauses, "Number of clauses does not match header");
        clauses.push([(Term{var: 0, negated: true}, TermState::Symbolic); CLAUSE_LENGTH]);  // Add a dummy clause to the end to make var 0 false
        assert!(clauses.iter().map(|c| c.iter().map(|(t, _)| t.var).max().unwrap()).max().unwrap() == var_count as u8, "Variable count does not match header");
        let num_clauses = clauses.len();
        let s = Self {
            clause_table: clauses,
            num_clauses: num_clauses,
            num_vars: (var_count+1) as usize
        };

        (s, sat)
    }
    
    pub fn write_file(&self, mut file: File) -> Result<(), std::io::Error> {
        
        // Write standard DIMACS CNF header comments
        file.write_all(b"c\n")?;
        file.write_all(b"c SAT instance in DIMACS CNF input format.\n")?;
        file.write_all(b"c\n")?;
        
        // Write the problem line with number of variables and clauses
        file.write_all(format!("p cnf {} {}\n", self.num_vars, self.num_clauses-1).as_bytes())?;
        
        // Write each clause
        let mut i = 0;
        for clause in &self.clause_table {
            if i == self.num_clauses-1{
                break;
            }
            i += 1;
            for (term, _) in clause {
                file.write_all(format!("{} ", if term.negated { -(term.var as i32) } else { term.var as i32 }).as_bytes())?;
            }
            file.write_all(b"0\n")?;
        }
        file.flush()?;
        
        Ok(())
    }

    pub fn number_of_vars(&self) -> usize {
        self.clause_table.iter().map(|c| c.iter().map(|(t, _)| t.var).max().unwrap()).max().unwrap() as usize
    }
}

impl Clone for ClauseTable {
    fn clone(&self) -> Self {
        Self { clause_table: self.clause_table.clone(), num_clauses: self.num_clauses, num_vars: self.num_vars }
    }
}