use std::vec;
use std::{fs::File, io::Write as IoWrite};
use std::{io::BufRead, path::PathBuf};
use std::rc::Rc;
use super::util_types::{NodeId, VarId, CLAUSE_LENGTH}; 
/// The symbolic symbol and its negation state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolicTerm {
    /// The variable id of the term
    pub var: VarId,
    /// The negation state of the term
    pub negated: bool,
}
/// The current assignment value of a term location. True/False/Symbolic
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermState {False, True, Symbolic}
impl Default for TermState {fn default() -> Self {TermState::Symbolic}}
/// The current assignment state of the terms in a clause. Made from an array of TermStates
pub type ClauseState = [TermState; CLAUSE_LENGTH];
/// An array of clauses representing the AND of the CNF formula
/// A CNF formula is a conjunction of clauses (ie (¬a ∨ b ∨ ¬c) ∧ (¬d ∨ e)) meaning ((¬a OR b OR ¬c) AND (¬d OR e))
pub type ProblemState = Vec<ClauseState>; 

/// The index of the term in the clause table
pub type TermLoc = (ClauseIdx, TermIdx); 
/// The index of the clause in the clause table
pub type ClauseIdx = usize; 
/// The index of the term in the clause (should be <CLAUSE_LENGTH)
pub type TermIdx = usize;

/// Term locations for a given symbol in the current clause table
#[derive(Clone, Debug, Default)]
pub struct TermLookup {
    /// List of all positive term locations (not negated)
    pub pos: Vec<TermLoc>, 
    /// List of all negative term locations (negated)
    pub neg: Vec<TermLoc>, 
}
/// Transpose table of clauses. Vector of TermLookups indexed by each variable id
pub type TransposeTable = Vec<TermLookup>; 
/// (ClauseIdx, TermIdx) -> Symbolic for unit propagation lookup
pub type SymbolicTable = Vec<[SymbolicTerm; CLAUSE_LENGTH]>;

// FIXME: the current BOX::leak looses memory over time
pub struct ClauseTable {
    /// Static (per problem) transpose table of clauses
    pub transpose: Rc<TransposeTable>,
    /// Static (per problem) symbolic table of clauses
    pub symbolic_table: Rc<SymbolicTable>,
    /// The dynamic assignment state of the clauses
    pub problem_state: Vec<ClauseState>,   
}

impl ClauseTable {
    fn check_table(&self, expected_clause_count: usize, expected_var_count: usize) {
        // There should be the same number of entries in the transpose table as in the problem state
        assert_eq!(
            self.transpose.iter().map(|TermLookup { pos, neg }| pos.len() + neg.len()).sum::<usize>(), 
            self.problem_state.len() * CLAUSE_LENGTH);

        assert_eq!(self.symbolic_table.len(), self.problem_state.len(), "Symbolic table does not match problem state");

        assert_eq!(self.problem_state.len(), expected_clause_count, "Clause count does not match header");
        assert_eq!(self.transpose.len(), expected_var_count, "Variable count does not match header");
    }

    fn build_symbolic_table(tranpose: &TransposeTable, num_clauses: usize) -> Vec<[SymbolicTerm; 3]> {
        let mut symbolic_table = vec![[SymbolicTerm { var: 0, negated: false }; CLAUSE_LENGTH]; num_clauses];
        for (var_id, term_lookup) in tranpose.iter().enumerate() {
            for (clause_idx, term_idx) in term_lookup.pos.iter() {
                symbolic_table[*clause_idx][*term_idx] = SymbolicTerm { var: var_id as VarId, negated: false };
            }
            for (clause_idx, term_idx) in term_lookup.neg.iter() {
                symbolic_table[*clause_idx][*term_idx] = SymbolicTerm { var: var_id as VarId, negated: true };
            }
        }
        symbolic_table
    }
    pub fn random(num_clauses: usize, num_vars: u8) -> Self {
        let problem_state: ProblemState = vec![[TermState::Symbolic; CLAUSE_LENGTH]; num_clauses+1];
        // TODO: handle var 0 for stated false (handle this for minisat and such)
        let mut transpose = vec![TermLookup { pos: Vec::new(), neg: Vec::new() }; num_vars as usize + 1];
        for clause_idx in 0..num_clauses {
            // let mut clause = [(Term{var: 0, negated: false}, TermState::Symbolic); CLAUSE_LENGTH];
            for term_idx in 0..CLAUSE_LENGTH {
                let var = ((rand::random::<u8>() % num_vars) + 1) as usize;  // FIXME: 1-indexed because of the way I am using the 0 var
                let negated = rand::random::<bool>();
                if negated {
                    transpose[var].neg.push((clause_idx, term_idx));
                } else {
                    transpose[var].pos.push((clause_idx, term_idx));
                }
            }
        }
        let num_clauses = problem_state.len();
        let symbolic_table = Self::build_symbolic_table(&transpose, num_clauses);
        let transpose = Rc::new(transpose); let symbolic_table = Rc::new(symbolic_table);
        Self {
            transpose,
            symbolic_table,
            problem_state,
        }
    }

    /// Load a file and return a new ClauseTable with expected SAT result
    /// Example File Format
    /// ```text
    /// c
    /// c SAT instance in DIMACS CNF input format.
    /// c
    /// p cnf 100 286                                           p cnf <num_vars> <num_clauses>
    /// 80  -39  -21  0                                         <var1> <var2> ... <varN> 0
    /// -58  25  23  0
    /// -88  55  -42  0
    /// -71  -49  46  0
    /// ```
    pub fn load_file(path: PathBuf) -> (Self, bool) {
        // Config parameters
        let mut clause_index = 0;
        let mut num_clauses = 0;
        let sat = !path.to_string_lossy().to_lowercase().contains("unsat");
        let mut var_count = 0;
        let mut transpose = vec![];

        // Read loop
        let file = std::fs::File::open(path).unwrap();
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap();
            let mut clause_end = false;
            if line.starts_with("p cnf") {  // Parse the number of variables and clauses *header*
                let mut parts = line.split_whitespace();
                parts.next(); // Skip "p"
                parts.next(); // Skip "cnf"
                var_count = parts.next().unwrap().parse().unwrap();
                assert!(var_count < u8::MAX as i32, "Too many variables for u8");
                num_clauses = parts.next().unwrap().parse().unwrap();
                transpose = vec![TermLookup { pos: Vec::new(), neg: Vec::new() }; var_count as usize + 1];  // FIXME: 1-indexed because of the way I am using the 0 var
            } else if line.starts_with("c") {  // Skip comments
                continue;
            } else if line.starts_with("%") {  // end this file
                break;
            } else {  // Parse the clauses
                let parts = line.split_whitespace();
                for (term_index, part) in parts.enumerate() {
                    assert!(term_index < CLAUSE_LENGTH, "Only 3SAT is supported");
                    let num: i32 = part.parse().unwrap();
                    if num == 0 {  // End of clause
                        break;
                    } else {
                        let var = num.abs() as usize;
                        let neg = num < 0;
                        assert!(var <= var_count as usize, "Variable {} is out of bounds", var);
                        if neg {
                            transpose[var].neg.push((clause_index, term_index));
                        } else {
                            transpose[var].pos.push((clause_index, term_index));
                        }
                    }
                }
                clause_index += 1;
            }
        }
        

        // clauses.push([(Term{var: 0, negated: true}, TermState::Symbolic); CLAUSE_LENGTH]);  // FIXME: dummy clause
        let symbolic_table = Self::build_symbolic_table(&transpose, num_clauses);
        let transpose = Rc::new(transpose); let symbolic_table = Rc::new(symbolic_table);
        let s = Self {
            transpose,
            symbolic_table,
            problem_state: vec![[TermState::Symbolic; CLAUSE_LENGTH]; num_clauses+1],
        };
        s.check_table(num_clauses, var_count as usize);
        (s, sat)
    }
    
    pub fn write_file(&self, mut file: File) -> Result<(), std::io::Error> {
        // Write standard DIMACS CNF header comments
        file.write_all(b"c\n")?;
        file.write_all(b"c SAT instance in DIMACS CNF input format.\n")?;
        file.write_all(b"c\n")?;
        
        // Write the problem line with number of variables and clauses
        let num_clauses = self.problem_state.len();
        let num_vars = self.transpose.len();
        file.write_all(format!("p cnf {} {}\n", num_vars, num_clauses-1).as_bytes())?;  // FIXME: this relies on our dummy clause at the end

        let mut i = 0;
        for clause in self.symbolic_table.iter() {  // TODO: change this to enumerate
            if i == num_clauses-1{  // FIXME: dummy clause handling
                break;
            }
            i += 1;
            for SymbolicTerm { var, negated } in clause.iter() {
                file.write_all(format!("{} ", if *negated { -(*var as i32) } else { *var as i32 }).as_bytes())?;
            }
            file.write_all(b"0\n")?;
        }
        file.flush()?;
        
        Ok(())
    }
    
    pub fn number_of_vars(&self) -> usize {
        self.transpose.len()
    }
    
    pub fn number_of_clauses(&self) -> usize {
        self.problem_state.len()
    }

    // TODO: add a fucntion from var_id to vec<Clause>
}

impl Clone for ClauseTable {
    fn clone(&self) -> Self {
        Self { 
            transpose: Rc::clone(&self.transpose),
            symbolic_table: Rc::clone(&self.symbolic_table),
            problem_state: self.problem_state.clone(), 
        }
    }
}
