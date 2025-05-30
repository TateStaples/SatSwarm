/*
# :microscope: microsat

A tiny (_microscopic_) DPLL SAT-solver written in Rust. This is not meant to be:

1. A particularly _fast_ solver
2. A particularly _extensible_ solver
3. A particularly _useful_ solver
https://github.com/RobScheidegger/microsat/blob/main/README.md
But instead serves as a proof-of-concept for what a small, readable, and understandable [DPLL](https://en.wikipedia.org/wiki/DPLL_algorithm) SAT Solver could look like in Rust.

This project originated as a project for Brown's [CSCI2951-O Foundations of Prescriptive Analysis](https://cs.brown.edu/courses/csci2951-o/).

Authors:

- [Rob Scheidegger ](https://github.com/RobScheidegger)
- [Hammad Izhar](https://github.com/Hammad-Izhar)

## Benchmarks

Although `microsat` isn't intended to be used as a fast SAT-solver, I felt it appropriate to compare it at a basic level to the project, [`minisat`](https://github.com/niklasso/minisat) (a small [CDCL](https://en.wikipedia.org/wiki/Conflict-driven_clause_learning) SAT solver that disrupted the SAT solver scene many years back). Times were for release-compiled variants of `microsat` and `minisat` on the same computer, for all of the examples in `examples/cnf`:

|| `microsat`  | `minisat`  |
|---|---|---|
|Time to solve example suite| 44.158s  |  41.432s |
|Lines of code| 791  | 3517 |

As you can see, `microsat` does pretty remarkably well in this benchmark, despite being _much_ smaller than the already small `minisat`. Further, it is important to note that for any reasonably large instance (e.g. larger than the `1040` variable, `3668` clause file in `examples/cnf`, which is the largest in this benchmark), so in a way, this benchmark is clearly cheating (but fascinating regardless).
*/

/*
Optimizations:
- SIMD
*/
use std::cmp::{max, min, Ordering};
use std::collections::BinaryHeap;
use std::fmt::{format, Debug, Display, Formatter};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::process::exit;
use hashbrown::{HashMap, HashSet, DefaultHashBuilder};
use bincode::{config, Decode, Encode};
use bincode::config::Configuration;
use crate::structures::trace::{save_log, Trace};

type Hash = DefaultHashBuilder;
fn hashmap<A, T>() -> HashMap<A, T, Hash> {HashMap::with_hasher(Default::default())}
fn hashset<A>() -> HashSet<A, Hash> {HashSet::with_hasher(Default::default())}
fn assignment() -> Assignment {Vec::new()}


#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Action {
    RemoveClause(ClauseId),
    RemoveLiteralFromClausesStart(),
    RemoveLiteralFromClause(ClauseId),
    RemoveLiteralFromClausesEnd(Literal),
    AssignVariable(Variable),
    SpeculateVariable(Variable),
}
/// The current value of each variable (I think they add both the pos and the neg to this)
pub type Assignment = Vec<Option<bool>>;
/// The index of the clause is the Expression (2^16 = ~64k)
pub type ClauseId = u16;
/// Symbolic Literal where negative means negated (2^25 = ~16k unique symbols)
pub type Literal = i16;
/// Variable name (I think because of _Literal_ they can only use 2^15)
pub type Variable = usize;

/// A symbolic clause with any number of literals OR'ed together (CNF form)
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct Clause {   
    /// The symbols that are in this clause
    pub variables: Vec<Literal>,
    pub enabled: bool
}

impl Clause { 
    pub fn new() -> Clause {
        Clause {
            variables: Vec::new(),
            enabled: true
        }
    }

    #[inline]
    pub fn insert_checked(&mut self, variable: Literal) {
        if !self.variables.contains(&variable) {
            self.variables.push(variable);
        }
    }

    #[inline]
    pub fn insert(&mut self, variable: Literal) {
        self.variables.push(variable);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.variables.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.variables.is_empty()
    }

    #[inline]
    pub fn literals(&self) -> &Vec<Literal> {
        &self.variables
    }

    #[inline]
    pub fn get(&self, index: usize) -> Literal {
        // self.variables[index]
        unsafe {
            *self.variables.get_unchecked(index)
        }
    }

    /// Efficient remove for a clause set that uses constant time by swapping the last element with the removed one.
    pub fn remove(&mut self, variable: Literal) {
        for i in 0..self.variables.len() {
            if self.variables[i] == variable {
                self.variables.swap_remove(i);
                return;
            }
        }
    }
}

#[inline]
pub fn to_variable(literal: Literal) -> Variable {
    literal.abs() as Variable
}

#[inline]
pub fn negate(variable: Literal) -> Literal {
    -variable
}

#[inline]
pub fn to_positive(variable: Literal) -> Literal {
    variable.abs()
}

/// Read in the SAT problem in standardized format
pub fn parse_dimacs(path: PathBuf) -> Expression {

    // Read the file from disk
    let mut cnf = Expression::new();
    let file: String = std::fs::read_to_string(&path).unwrap();

    // Read each line of the file
    for line in file.lines() {
        // If the line starts with 'c', then it is a comment, so skip it
        if line.starts_with('c') || line.is_empty() || line.starts_with('%') {
            continue;
        }

        // If the line starts with 'p', then it is a problem line
        if line.starts_with('p') {
            let mut parts = line.split_whitespace();
            let _ = parts.next(); // Skip the 'p'
            let _ = parts.next(); // Skip the 'cnf'
            let num_vars: Variable = parts.next().unwrap().parse().unwrap();
            cnf.assignments = vec![None; num_vars];
            let num_clauses: ClauseId = parts.next().unwrap().parse().unwrap();
            continue;
        }

        // Otherwise, the line is a clause
        let mut clause = Clause::new();
        for literal in line.split_whitespace() {
            let value = literal.parse::<Literal>().unwrap();
            if value == 0 {
                break;
            }
            clause.insert_checked(value);
        }

        cnf.add_clause(clause);
    }

    cnf
}

pub type TraceId = usize;
pub fn solve_dpll(cnf: &mut Expression, log: &mut Vec<Trace>) {
    // Track where we are in the action stace
    debug_assert!(cnf.clauses.iter().all(|clause| clause.len()>0));
    debug_assert_eq!(cnf.clauses.iter().filter(|clause| clause.enabled).count(), cnf.num_active_clauses as usize);
    cnf.checks();
    // Try to do as much inference as we can before branching
    while let Some(unit_prop) = cnf.pop_unit_clause() {
        if let Some(unsat_clause) = cnf.remove_unit_clause(unit_prop) {
            cnf.unit_clauses.clear();
            let unit_props = cnf.backtrack();
            let trace = Trace::unsat(unit_props, unsat_clause);
            log.push(trace);
            return;
        }
    }
    cnf.checks();
    if cnf.is_satisfied() {
        let trace = Trace::sat(cnf.backtrack());
        log.push(trace);
        println!("SAT!");
        debug_assert!(log.last().unwrap().is_sat());
        return;
    }
    debug_assert!(cnf.assignments.iter().any(|x| x.is_none()));
    debug_assert!(cnf.unit_clauses.is_empty());

    // Have to BRANCH!
    let mut placeholder = Trace::placeholder();
    let my_trace_idx = log.len();
    log.push(placeholder);
    
    // Pick some variable to branch on ("guess") to keep searching
    let (branch_variable, branch_value) = cnf.get_branch_variable();

    // Try the first branch value (don't track idx because always immediately after current
    if let Some(unsat_clause) = cnf.branch_variable(branch_variable, branch_value) {
        let unit_props = cnf.backtrack();
        let left_trace = Trace::unsat(unit_props, unsat_clause);
        log.push(left_trace);
    } else {
        solve_dpll(cnf, log);
        if log.last().unwrap().is_sat() {
            let unit_props = cnf.backtrack();
            let my_trace = unsafe{log.get_unchecked_mut(my_trace_idx)};
            *my_trace = Trace::branch(unit_props, 0);
            return;
        }
    }  // by this point the trace after my_trace should be left_trace either explicit or recursive
    debug_assert!(log.len() > my_trace_idx);
    debug_assert_eq!(cnf.clauses.iter().filter(|clause| clause.enabled).count(), cnf.num_active_clauses as usize);

    // Try the other branch value
    let right_trace_idx = if let Some(unsat_clause) = cnf.branch_variable(branch_variable, !branch_value) {
        let unit_props = cnf.backtrack();
        let right_trace = Trace::unsat(unit_props, unsat_clause);
        let right_trace_idx = log.len();
        log.push(right_trace);
        right_trace_idx
    } else {
        let right_trace_idx = log.len();
        solve_dpll(cnf, log);
        right_trace_idx
    };
    debug_assert!(log.len() >= right_trace_idx && right_trace_idx > my_trace_idx+1);
    debug_assert_eq!(cnf.clauses.iter().filter(|clause| clause.enabled).count(), cnf.num_active_clauses as usize);

    let unit_props = cnf.backtrack();
    let my_trace = unsafe{log.get_unchecked_mut(my_trace_idx)};
    *my_trace = Trace::branch(unit_props, right_trace_idx);
}


#[derive(Clone, Copy, Debug)]
pub enum SolverHeuristic {
    FirstVariable,
    MostLiteralOccurrences,
    MostVariableOccurrences,
    MinimizeClauseLength,
}
/// SAT problem
pub struct Expression {
    /// All of the Clauses AND'ed in CNF form
    clauses: Vec<Clause>,
    /// Action history (most recent at the top of the stack)
    actions: Vec<Action>,
    /// The final assignment values of each variable
    assignments: Assignment,
    /// Transposed problem listing where each variable occurs (note -1 and 1 are considered different variables)
    literal_to_clause: HashMap<Literal, HashSet<ClauseId, Hash>, Hash>,
    /// Currently identified unit_clauses TODO: this doesn't maintain ordering we would probably want from our unit clauses
    unit_clauses: BinaryHeap<ClauseId>,
    /// Tracks when the problem is done
    pub num_active_clauses: u16,
    /// Limits the k-SAT
    max_clause_length: usize,
    /// Variable decision procedure
    pub heuristic: SolverHeuristic,
}

impl Clone for Expression {
    fn clone(&self) -> Self {
        let mut new_expression = Expression::new();
        for clause in &self.clauses {
            new_expression.add_clause(clause.clone());
        }

        new_expression
    }
}

impl Expression {
    pub fn new() -> Expression {
        Expression {
            clauses: Vec::new(),
            actions: Vec::new(),
            assignments: assignment(),

            literal_to_clause: hashmap(),
            unit_clauses: BinaryHeap::new(),
            num_active_clauses: 0,
            max_clause_length: 0,
            heuristic: SolverHeuristic::FirstVariable,
        }
    }

    pub fn from_clauses(clauses: Vec<Clause>) -> Expression {
        let mut expression = Expression::new();
        for clause in clauses {
            expression.add_clause(clause);
        }

        expression
    }

    pub fn from_cnf_file(file_name: &str) -> Expression {
        let path: PathBuf = PathBuf::from(file_name);
        parse_dimacs(path)
    }

    fn get_clauses(&self) -> Vec<Clause> {
        self.clauses.clone()
    }

    pub fn set_heuristic(&mut self, heuristic: SolverHeuristic) {
        self.heuristic = heuristic;
    }

    fn find_literal(&self, clause_id: ClauseId, literal_id: usize) -> Literal {
        Self::get_clause(&self.clauses, clause_id).get(literal_id)
    }

    fn get_clause(clauses: &Vec<Clause>, clause_id: ClauseId) -> &Clause {
        // &clauses[clause_id as usize]
        unsafe {
            &clauses.get_unchecked(clause_id as usize)
        }
    }
    fn get_mut_clause(clauses: &mut Vec<Clause>, clause_id: ClauseId) -> &mut Clause {
        // &mut clauses[clause_id as usize]
        unsafe {
            clauses.get_unchecked_mut(clause_id as usize)
        }
    }

    /// Softly removes a clause from the expression.
    /// This means that the clause is not actually removed from the expression vector,
    /// but all references to it have been removed from the literal map, so it is unreferenced.
    fn remove_clause(&mut self, clause_id: ClauseId) {
        self.checks();
        // Remove all of the literals in the clause from the variable_to_clause map
        let clause = Self::get_mut_clause(&mut self.clauses, clause_id);
        debug_assert!(clause.enabled);
        self.num_active_clauses -= 1;
        clause.enabled = false;
        for i in 0..clause.len() {
            let literal = self.find_literal(clause_id, i);
            let literal_clauses = self.literal_to_clause.get_mut(&literal).unwrap();
            literal_clauses.remove(&clause_id);
        }
        self.checks();
        self.unit_clauses.retain(|x|x != &clause_id);  // TODO: this is inefficient
        self.actions.push(Action::RemoveClause(clause_id));
    }

    /// Re-enables a clause that had been softly removed, so all of its literals are still present in the vector.
    fn enable_clause(&mut self, clause_id: ClauseId) {
        let Self {literal_to_clause, unit_clauses, clauses, ..} = self;
        let clause = Self::get_mut_clause(clauses, clause_id);
        debug_assert!(!clause.enabled);
        clause.enabled = true;
        self.num_active_clauses += 1;
        for i in 0..clause.len() {
            let literal = clause.get(i);
            let literal_clauses = literal_to_clause.get_mut(&literal).unwrap();
            literal_clauses.insert(clause_id);
        }
        // if clause.len() == 1 {
        //     // unit_clauses.add(clause_id.clone());
        //     unit_clauses.push(clause_id);
        // }
    }
    #[inline]
    fn checks(&self) {
        debug_assert!({
            let mut result = true;
            for (lit, clauses) in &self.literal_to_clause {
                if self.get_literal(*lit).is_some() {
                    // result = true;
                    // break;
                    continue;
                }
                for clause_id in clauses {
                    let clause = &self.clauses[*clause_id as usize];
                    if !clause.enabled || !clause.variables.contains(lit) {
                        println!("Break 2");
                        result = false;
                        break;
                    }
                }
                if !result { break; }
            }
            result
            // self.literal_to_clause.iter().all(|(lit, clauses)| {
            //     clauses.iter()
            //         .map(|id| &self.clauses[*id as usize])
            //         .all(|c| c.enabled
            //             && c.variables.contains(&lit))
            //         || self.assignments[to_variable(*lit)].is_some()
            // })
        });
        debug_assert_eq!(self.clauses.iter().filter(|clause| clause.enabled).count(), self.num_active_clauses as usize);
        debug_assert!(self.clauses.iter().all(|clause| !clause.is_empty()));
    }

    /// Removes a literal from all of the clauses that it is in
    fn remove_literal_from_clauses(&mut self, literal: Literal) -> Option<ClauseId> {
        let clauses_result = self.literal_to_clause.get(&literal);
        if clauses_result.is_none() {  // TODO: why would this
            return None;
        }
        self.checks();
        let actions = &mut self.actions;
        actions.push(Action::RemoveLiteralFromClausesStart());

        let literal_clauses = clauses_result.unwrap();
        for clause_id in literal_clauses {
            let clause = &mut self.clauses[*clause_id as usize];
            clause.remove(literal);
            actions.push(Action::RemoveLiteralFromClause(*clause_id));

            if clause.is_empty() {  // UNSAT
                clause.variables.push(literal);
                actions.pop();
                actions.push(Action::RemoveLiteralFromClausesEnd(literal));
                self.checks();
                return Some(*clause_id);
            }

            if clause.len() == 1 {
                self.unit_clauses.push(*clause_id);
            }
        }
        actions.push(Action::RemoveLiteralFromClausesEnd(literal));
        self.checks();
        return None;
    }

    /// Removes all clauses with the specified literal.
    fn remove_clauses_with_literal(&mut self, literal: Literal) {
        self.checks();
        let literal_clauses;
        {
            let literal_clauses_ref = self.literal_to_clause.get(&literal);
            if literal_clauses_ref.is_none() {
                return;
            }
            literal_clauses = literal_clauses_ref.unwrap().clone();
        }
        self.checks();
        for clause_id in literal_clauses {
            self.remove_clause(clause_id);
        }
    }

    fn assign_variable(&mut self, variable: Variable, value: bool, spec: bool) -> Option<ClauseId> {
        self.checks();
        self.set_variable(variable, value);
        // Add to action history for potential future undoing
        let action = if spec {Action::SpeculateVariable(variable)} else { Action::AssignVariable(variable) };
        self.actions.push(action);
        let literal = if value {
            variable as Literal
        } else {
            -(variable as Literal)
        };
        let negated_literal = negate(literal);
        if let Some(unsat_clause) = self.remove_literal_from_clauses(negated_literal) {
            return Some(unsat_clause);
        }
        self.checks();

        self.remove_clauses_with_literal(literal);  // Remove Trues
        self.checks();
        return None;
    }

    #[inline]
    fn unassign_variable(&mut self, variable: Variable) {
        // self.assignments[variable-1] = None;
        debug_assert!(self.assignments[variable-1].is_some());
        unsafe {
            *self.assignments.get_unchecked_mut(variable-1) = None;
        }
    }

    pub fn optimize(&mut self) {
        self.clauses.retain(|clause| !clause.is_empty());  // Remove empty clauses
        self.num_active_clauses = self.clauses.len() as ClauseId;
        // assert!(self.assignments.iter().all(|assignment| assignment.is_none()));
        // // Order literals by frequency (most common are the lowest)
        // let mut counts = hashmap();
        // let mut variables: Vec<Variable> = (1..self.assignments.len()+1).collect();
        // for variable in variables.clone() {
        //     let count = self.clauses.iter()
        //         .map(|clause| clause.literals().iter()
        //             .filter(|x| to_variable(**x)==variable))
        //             .count();
        //     counts.insert(variable, count);
        // }
        // variables.sort_by(|a, b| counts.get(a).unwrap().partial_cmp(counts.get(b).unwrap()).unwrap());
        // let prioritization: HashMap<Variable, Variable, Hash> = variables.into_iter().enumerate().map(|(i, var)| (i+1, var)).collect();
        // self.literal_to_clause = self.literal_to_clause.clone().into_iter().map(|(literal, set)| {
        //     let new_var = prioritization.get(&to_variable(literal)).unwrap();
        //     let new_literal = if literal>0 {*new_var as Literal} else {-(*new_var as Literal)};
        //     (new_literal, set)
        // }).collect();
        // self.clauses.iter_mut().map(|clause| {
        //     clause.variables.iter_mut().map(|literal| {
        //         let new_var = prioritization.get(&to_variable(*literal)).unwrap();
        //         let new_literal = if *literal>0 {*new_var as Literal} else {-(*new_var as Literal)};
        //         *literal = new_literal;
        //     })
        // });

        self.actions = Vec::with_capacity(self.clauses.len() * self.max_clause_length); // Pre-allocate a reasonable amount of space
    }

    pub fn is_satisfied_by(&self, assignment: &Assignment) -> bool {
        for clause in &self.clauses {
            let mut satisfied = false;
            for literal in clause.literals() {
                let variable = to_variable(*literal);
                if let Some(value) = self.get_variable(variable) {
                    if *value == (*literal>0) {  // TODO: might be replaceable with get_literal
                        satisfied = true;
                        break;
                    }
                }
            }

            if !satisfied {
                return false;
            }
        }

        true
    }

    #[inline]
    fn get_variable(&self, variable: Variable) -> &Option<bool> {
        // self.assignments[variable - 1]
        debug_assert!(self.assignments[variable-1].or(Some(true)).is_some());
        unsafe {
            self.assignments.get_unchecked(variable-1)
        }
    }
    #[inline]
    fn get_literal(&self, literal: Literal) -> Option<bool> {
        match self.get_variable(to_variable(literal)) {
            Some(b) => Some(*b == (literal>0)),
            None => None,
        }
    }
    #[inline]
    fn set_variable(&mut self, variable: Variable, value: bool) {
        // self.assignments[variable-1] = Some(value);
        debug_assert!(self.assignments[variable-1].is_none());
        unsafe {
            *self.assignments.get_unchecked_mut(variable-1) = Some(value);
        }
    }

    fn get_most_literal_occurrences(&self) -> (Variable, bool) { unimplemented!() }

    fn get_most_variable_occurrences(&self) -> (Variable, bool) { unimplemented!() }

    fn add_clause(&mut self, clause: Clause) {
        let clause_id = self.clauses.len() as ClauseId;

        for literal in clause.literals() {
            let excess: i32 = to_variable(*literal) as i32 - self.assignments.len() as i32;
            if excess > 0 {
                self.assignments.extend(vec![None; excess as usize]);
            }

            let variable: Variable = to_variable(*literal);

            if !self.literal_to_clause.contains_key(literal) {
                self.literal_to_clause.insert(*literal, hashset());
            }

            if !self.literal_to_clause.contains_key(&negate(*literal)) {
                self.literal_to_clause
                    .insert(negate(*literal), hashset());
            }

            let literal_clauses = self.literal_to_clause.get_mut(literal).unwrap();
            literal_clauses.insert(clause_id);

            // Check if the literal is a pure literal
            // self.check_pure_literal(*literal);
        }

        // Make sure we add it if it is a unit clause
        if clause.len() == 1 {
            // self.unit_clauses.insert(clause_id);
            self.unit_clauses.push(clause_id);
        }

        if clause.len() > self.max_clause_length {
            self.max_clause_length = clause.len();
        }

        self.clauses.push(clause);
        self.num_active_clauses += 1;
        debug_assert!(self.num_active_clauses == self.clauses.len() as ClauseId);
    }

    fn pop_unit_clause(&mut self) -> Option<ClauseId> {
        if self.unit_clauses.is_empty() {  // if there is nothing to unit propagate
            return None;
        }
        self.unit_clauses.pop()
    }
    fn remove_unit_clause(&mut self, clause_id: ClauseId) -> Option<ClauseId> {
        self.checks();
        let literal = self.find_literal(clause_id, 0);
        self.assign_variable(to_variable(literal), literal > 0, false)
    }

    fn construct_assignment(&mut self) -> Assignment {
        self.assignments.clone()
    }

    #[inline]
    fn is_satisfied(&self) -> bool {
        self.num_active_clauses == 0
    }
    

    fn backtrack(&mut self) -> u16 {
        debug_assert_eq!(self.clauses.iter().filter(|clause| clause.enabled).count(), self.num_active_clauses as usize);
        self.unit_clauses.clear();
        let mut unit_prop_count = 0;
        while let Some(action) = self.actions.pop() {
            match action {
                Action::RemoveClause(clause_id) => self.enable_clause(clause_id),  // Removed when one of the variables is set to true
                Action::RemoveLiteralFromClausesEnd(literal) => {  // gonna now have a series of literal removals
                    let removing_literal_clauses = self.literal_to_clause.get_mut(&literal).unwrap();

                    let mut should_exit = false;

                    while !should_exit {
                        let next_action = (&mut self.actions).pop().expect("Did not encounter a start literal!");
                        match next_action {
                            Action::RemoveLiteralFromClause(clause_id) => {
                                let clause = Self::get_mut_clause(&mut self.clauses, clause_id);
                                debug_assert!(!clause.variables.contains(&literal));
                                clause.insert(literal);
                                removing_literal_clauses.insert(clause_id);
                            }
                            Action::RemoveLiteralFromClausesStart() => {
                                should_exit = true;
                            }
                            _ => panic!("Did not encounter a start literal!"),
                        }
                    }
                }
                Action::AssignVariable(variable) => {
                    self.unassign_variable(variable);
                    unit_prop_count += 1;
                }
                Action::SpeculateVariable(variable) => {
                    self.unassign_variable(variable);
                    self.checks();
                    return unit_prop_count;
                }
                _ => unreachable!(),  // was break but i don't see how you should get here
            }
        }
        self.checks();
        unit_prop_count
    }
    
    fn get_branch_variable(&self) -> (Variable, bool) {
        match self.heuristic {
            SolverHeuristic::FirstVariable => (self.assignments.iter().position(|x| x.is_none()).unwrap() + 1, false),
            SolverHeuristic::MostLiteralOccurrences => self.get_most_literal_occurrences(),
            SolverHeuristic::MostVariableOccurrences => self.get_most_variable_occurrences(),
            SolverHeuristic::MinimizeClauseLength => {
                unreachable!("Got rid of this cause seemed hard to hardware implement")
            }
        }
    }

    fn branch_variable(&mut self, variable: Variable, value: bool) -> Option<ClauseId> {
        self.assign_variable(variable, value, true)
    }
}

fn verify_assignment(expression: &Expression, assignment: &Assignment) -> bool {  
    expression.is_satisfied_by(assignment)
}

fn solve(expression: Expression, log: &mut Vec<Trace>) {
    let mut modifiable = expression.clone();
    // Old code would multithread another dpll with MinimizeClauseLength heuristic on clone of expression
    modifiable.optimize(); 
    modifiable.set_heuristic(SolverHeuristic::FirstVariable);
    debug_assert!(modifiable.clauses.iter().all(|c| c.len()>0));
    solve_dpll(&mut modifiable, log);
}

// Tests
pub fn build_trace_path(path: PathBuf) {
    println!("Building trace path: {}", path.display());
    let trace_path = format!("traces/trace_of_{}", path.file_name().unwrap().to_str().unwrap());
    if File::open(&trace_path).is_ok() {
        println!("Trace already exists! Skipping...");
        return;
    }
    let expression = Expression::new();
    let expression = parse_dimacs(path.clone());
    let mut log = Vec::with_capacity(50_000_000);
    let start_time = std::time::Instant::now();
    solve(expression, &mut log);
    assert_ne!(log.last().unwrap().is_sat(), path.to_str().unwrap().contains("unsat"));
    println!("Solved in {} seconds with {} branches", start_time.elapsed().as_secs_f64(), log.len());
    save_log(log, trace_path);
}

pub fn main() {
    // let tp = "/Users/tatestaples/Code/SatSwarm/traces/trace_of_uuf50-099.cnf";
    // let trace = load_log(String::from(tp));
    // println!("{:?}", trace);
    // exit(1);
    println!("the very beginning");
    let path = "/Users/tatestaples/Code/SatSwarm/tests/satlib/unsat/uuf50-099.cnf";
    // let path = "/Users/tatestaples/Code/SatSwarm/tests/satlib/sat/uf50-099.cnf";
    let expression = Expression::from_cnf_file(path);
    let mut log = Vec::with_capacity(50_000_000);
    println!("starting");
    println!("Active clauses: {}", expression.num_active_clauses);
    let start_time = std::time::Instant::now();
    let result = solve(expression, &mut log);
    println!("Time: {}", start_time.elapsed().as_secs_f64());
    println!("Log Len {} and size {}", log.len(), log.len()*size_of::<Trace>());
    assert_ne!(log.last().unwrap().is_sat(), path.contains("unsat"));

    let trace_path = format!("traces/trace_of_{}", path.split('/').last().unwrap());
    save_log(log, trace_path);
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_verify_assignment() {
    //     let mut expression = Expression::new();
    //     let mut clause = Clause::new();
    //     clause.insert_checked(1);
    //     clause.insert_checked(-2);
    //     expression.add_clause(clause);
    //
    //     let mut assignment = assignment();
    //     assignment.insert(1, true);
    //     assignment.insert(2, false);
    //
    //     assert!(verify_assignment(&expression, &assignment));
    // }
    //
    // #[test]
    // fn test_verify_assignment_unsatisfied() {
    //     let mut expression = Expression::new();
    //     let mut clause = Clause::new();
    //     clause.insert_checked(1);
    //     clause.insert_checked(2);
    //     expression.add_clause(clause);
    //
    //     let mut assignment = assignment();
    //     assignment.insert(1, false);
    //     assignment.insert(2, false);
    //
    //     assert!(!verify_assignment(&expression, &assignment));
    // }
    //
    // #[test]
    // fn test_verify_assignment_unsatisfied_multiple_clauses() {
    //     let mut expression = Expression::new();
    //     let mut clause = Clause::new();
    //     clause.insert_checked(1);
    //     clause.insert_checked(2);
    //     expression.add_clause(clause);
    //
    //     let mut clause = Clause::new();
    //     clause.insert_checked(-3);
    //     clause.insert_checked(-4);
    //     expression.add_clause(clause);
    //
    //     let mut assignment = assignment();
    //     assignment.insert(1, false);
    //     assignment.insert(2, false);
    //     assignment.insert(3, true);
    //     assignment.insert(4, true);
    //
    //     assert!(!verify_assignment(&expression, &assignment));
    // }
    //
    // #[test]
    // fn test_verify_assignment_satisfied_multiple_clauses() {
    //     let mut expression = Expression::new();
    //     let mut clause = Clause::new();
    //     clause.insert_checked(1);
    //     clause.insert_checked(-2);
    //     expression.add_clause(clause);
    //
    //     let mut clause = Clause::new();
    //     clause.insert_checked(3);
    //     clause.insert_checked(-4);
    //     expression.add_clause(clause);
    //
    //     let mut assignment = assignment();
    //     assignment.insert(1, true);
    //     assignment.insert(2, false);
    //     assignment.insert(3, true);
    //     assignment.insert(4, false);
    //
    //     assert!(verify_assignment(&expression, &assignment));
    // }
}