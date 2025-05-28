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
- XOR clause form
- less variable search?
*/
use std::cmp::{max, min, Ordering};
use hashbrown::{HashMap, HashSet, DefaultHashBuilder};

type Hash = DefaultHashBuilder;
fn hashmap<A, T>() -> HashMap<A, T, Hash> {HashMap::with_hasher(Default::default())}
fn hashset<A>() -> HashSet<A, Hash> {HashSet::with_hasher(Default::default())}
fn assignment() -> Assignment {Vec::new()}

struct Trace {
    unit_props: ClauseId,
    // Some sparse memory traversal information
    // Either left_child (usize/u32) or clause_idx (u16) of unsat +
}
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Action {
    RemoveClause(ClauseId),
    RemoveLiteralFromClausesStart(),
    RemoveLiteralFromClause(ClauseId),
    RemoveLiteralFromClausesEnd(Literal),
    AssignVariable(Variable),
}
/// The current value of each variable (I think they add both the pos and the neg to this)
pub type Assignment = Vec<Option<bool>>;  // TODO: Idk if this is ever used, more efficient if we don't
/// The index of the clause is the Expression (2^16 = ~64k)
pub type ClauseId = u16;
/// Symbolic Literal where negative means negated (2^25 = ~16k unique symbols)
pub type Literal = i16;
/// Variable name (I think because of _Literal_ they can only use 2^15)
pub type Variable = usize;
/// How far into the action stack 
pub type ActionState = usize;

/// A symbolic clause with any number of literals OR'ed together (CNF form)
#[derive(Debug, Default, Eq, PartialEq, Clone)]
pub struct Clause {   // TODO: maybe try the XOR form suggested in the paper
    /// The symbols that are in this clause
    variables: Vec<Literal>,
}
/// Trait representing the important modifications of the CNF form
pub trait CNF {
    /// Adds a new clause to the CNF representation.
    fn add_clause(&mut self, clause: Clause);

    /// Removes a unit clause (if it exists) from the CNF and returns it.
    fn remove_unit_clause(&mut self) -> Option<ClauseId>;

    /// Removes a pure literal (if it exists) from the CNF and returns it.
    // fn remove_pure_literal(&mut self) -> Option<Literal>;

    /// Constructs an assignment from the current state of the CNF.
    /// This is only valid if the CNF is satisfiable.
    fn construct_assignment(&mut self) -> Assignment;

    /// Returns true if the CNF is satisfiable.
    fn is_satisfied(&self) -> bool;

    fn is_unsatisfiable(&self) -> bool;

    /// Current length of action history
    fn get_action_state(&self) -> ActionState;

    /// Restore to past point of action history (by undoing actions)
    fn restore_action_state(&mut self, state: ActionState);

    fn is_inference_possible(&self) -> bool;

    /// Decide on what variable to branch on
    fn get_branch_variable(&self) -> (Variable, bool);

    /// Perform branching with a particular variable and assignment
    fn branch_variable(&mut self, variable: Variable, value: bool);
}

impl Clause {  // FIXME: why is the implementation of clause seperated from the struct definition
    pub fn new() -> Clause {
        Clause {
            variables: Vec::new(),
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

    // #[inline]
    // pub fn contains(&self, variable: Literal) -> bool {
    //     self.variables.contains(&variable)
    // }

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
pub fn parse_dimacs(filename: &str) -> Expression {

    // Read the file from disk
    let mut cnf = Expression::new();
    let file = std::fs::read_to_string(filename).unwrap();

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


pub fn solve_dpll(cnf: &mut Expression) -> (Option<Assignment>, u32) {
    // Track where we are in the action stack
    let action_state: ActionState = cnf.get_action_state();

    // Try to do as much inference as we can before branching
    while cnf.is_inference_possible() {
        // FIXME: feel like it may be more efficient to test for failure on assignment
        // Next, remove all of the unit clauses
        while cnf.remove_unit_clause().is_some() {
            // TODO: count these for the log
        }

        // If the CNF is satisfied, then we are done
        if cnf.is_unsatisfiable() {
            // Restore the action state (undo branching)
            cnf.restore_action_state(action_state);
            return (None, 0);
        }

        // while cnf.remove_pure_literal().is_some() {}
    }

    if cnf.is_satisfied() {
        return (Some(cnf.construct_assignment()), 0);
    }

    if cnf.is_unsatisfiable() {
        cnf.restore_action_state(action_state);
        return (None, 1);
    }

    // TODO: log branch
    // Pick some variable to branch on ("guess") to keep searching
    let branch_action_state = cnf.get_action_state();
    let (branch_variable, branch_value) = cnf.get_branch_variable();
    
    // Try the first branch value
    cnf.branch_variable(branch_variable, branch_value);

    let (branch_result, branches) = solve_dpll(cnf);
    if branch_result.is_some() {
        return (branch_result, branches+1);
    }

    cnf.restore_action_state(branch_action_state);

    // Try the other branch value
    cnf.branch_variable(branch_variable, !branch_value);

    let (branch_result, more_branches) = solve_dpll(cnf);
    if branch_result.is_some() {
        return (branch_result, branches+more_branches+1);
    }

    cnf.restore_action_state(action_state);
    (None, branches+more_branches+1)
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
    unit_clauses: HashSet<ClauseId, Hash>,
    /// Literals (Var + assignment) that only have one polarity
    // pure_literals: HashSet<Literal>,
    /// Tracks when the problem is done
    pub num_active_clauses: u16,
    /// Tracks how much left of the problem (presumable active + empty is constant)
    num_empty_clauses: usize,
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

impl Default for Expression {
    fn default() -> Self {
        Self::new()
    }
}

impl Expression {
    pub fn new() -> Expression {
        Expression {
            clauses: Vec::new(),
            actions: Vec::new(),
            assignments: assignment(),

            literal_to_clause: hashmap(),
            unit_clauses: hashset(),
            // pure_literals: HashSet::new(),
            num_active_clauses: 0,
            num_empty_clauses: 0,
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
        parse_dimacs(file_name)
    }

    pub fn get_clauses(&self) -> Vec<Clause> {
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
        // Remove all of the literals in the clause from the variable_to_clause map
        let clause = Self::get_clause(&self.clauses, clause_id);
        for i in 0..clause.len() {
            let literal = self.find_literal(clause_id, i);
            let literal_clauses = self.literal_to_clause.get_mut(&literal).unwrap();
            literal_clauses.remove(&clause_id);
        }
        self.num_active_clauses -= 1;
        self.unit_clauses.remove(&clause_id);
        self.actions.push(Action::RemoveClause(clause_id));
    }

    /// Re-enables a clause that had been softly removed, so all of its literals are still present in the vector.
    fn enable_clause(&mut self, clause_id: ClauseId) {
        self.num_active_clauses += 1;
        
        let Self {literal_to_clause, unit_clauses, clauses, ..} = self;
        let clause = Self::get_clause(clauses, clause_id);
        for i in 0..clause.len() {
            let literal = clause.get(i);
            let literal_clauses = literal_to_clause.get_mut(&literal).unwrap();
            literal_clauses.insert(clause_id);
        }
        if clause.len() == 1 {
            unit_clauses.insert(clause_id.clone());
        }
    }

    /// Removes a literal from all of the clauses that it is in
    fn remove_literal_from_clauses(&mut self, literal: Literal) {
        let clauses_result = self.literal_to_clause.get(&literal);
        if clauses_result.is_none() {
            return;
        }

        // let actions = self.actions.clone();
        let actions = &mut self.actions;
        actions.push(Action::RemoveLiteralFromClausesStart());

        let literal_clauses = clauses_result.unwrap();
        for clause_id in literal_clauses {
            let clause = &mut self.clauses[*clause_id as usize];
            clause.remove(literal);

            if clause.len() == 1 {
                self.unit_clauses.insert(*clause_id);
            }

            if clause.is_empty() {
                self.num_empty_clauses += 1;
                self.unit_clauses.remove(clause_id);
            }

            actions.push(Action::RemoveLiteralFromClause(*clause_id));
        }

        actions.push(Action::RemoveLiteralFromClausesEnd(literal));
    }

    /// Removes all clauses with the specified literal.
    fn remove_clauses_with_literal(&mut self, literal: Literal) {
        let literal_clauses;
        {
            let literal_clauses_ref = self.literal_to_clause.get(&literal);
            if literal_clauses_ref.is_none() {
                return;
            }
            literal_clauses = literal_clauses_ref.unwrap().clone();
        }

        for clause_id in literal_clauses {
            self.remove_clause(clause_id);
        }
    }
    
    fn assign_variable(&mut self, variable: Variable, value: bool) {
        self.set_variable(variable, value);
        // Add to action history for potential future undoing
        self.actions.push(Action::AssignVariable(variable));
        let literal = if value {
            variable as Literal
        } else {
            -(variable as Literal)
        };
        let negated_literal = negate(literal);
        self.remove_clauses_with_literal(literal);  // Remove Trues
        self.remove_literal_from_clauses(negated_literal);  // Shrink false
    }

    #[inline]
    fn unassign_variable(&mut self, variable: Variable) {
        // self.assignments[variable-1] = None;
        debug_assert!(self.assignments[variable-1].is_none_or(|_| true));
        unsafe {
            *self.assignments.get_unchecked_mut(variable-1) = None;
        }
    }

    pub fn optimize(&mut self) {
        self.clauses.retain(|clause| !clause.is_empty());  // Remove empty clauses
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
        debug_assert!(self.assignments[variable-1].is_none_or(|_| true));
        unsafe {
            *self.assignments.get_unchecked_mut(variable-1) = Some(value);
        }
    }

    fn get_most_literal_occurrences(&self) -> (Variable, bool) { todo!() }

    fn get_most_variable_occurrences(&self) -> (Variable, bool) { todo!() }
}

impl CNF for Expression {
    fn add_clause(&mut self, clause: Clause) {
        let clause_id = self.clauses.len() as ClauseId;

        for literal in clause.literals() {
            let excess: i32 = to_variable(*literal) as i32 - self.assignments.len() as i32;
            // FIXME
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
            self.unit_clauses.insert(clause_id);
        }

        if clause.len() > self.max_clause_length {
            self.max_clause_length = clause.len();
        }

        self.clauses.push(clause);
        self.num_active_clauses += 1;
    }

    fn remove_unit_clause(&mut self) -> Option<ClauseId> {  // FIXME: weird return type considering its not used on its only call
        if self.unit_clauses.is_empty() {  // if there is nothing to unit propagate
            return None;
        }
        // Interesting they store unit_props as clauses instead of literals -> wonder why
        let clause_id: ClauseId = *self.unit_clauses.iter().next().unwrap();  // constantly making an iter seems inefficient FIXME

        // Get the *only* element left in the clause
        let literal = self.find_literal(clause_id, 0);

        self.assign_variable(to_variable(literal), literal > 0);
        // The clause of the unit propagation
        Some(clause_id)
    }

    // fn remove_pure_literal(&mut self) -> Option<Literal> {
    //     if self.pure_literals.is_empty() {
    //         return None;
    //     }
    // 
    //     let literal: Literal = *self.pure_literals.iter().next().unwrap();
    // 
    //     self.assign_variable(to_variable(literal), literal > 0);
    //     Some(literal)
    // }

    fn construct_assignment(&mut self) -> Assignment {
        self.assignments.clone()
    }

    #[inline]
    fn is_satisfied(&self) -> bool {
        self.num_active_clauses == 0
    }

    #[inline]
    fn is_unsatisfiable(&self) -> bool {
        self.num_empty_clauses > 0
    }

    fn get_action_state(&self) -> ActionState {
        self.actions.len()
    }
    
    fn restore_action_state(&mut self, state: ActionState) {
        while self.actions.len() > state {
            let action = (&mut self.actions).pop().unwrap();
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
                                clause.insert(literal);
                                if clause.len() == 1 {
                                    self.num_empty_clauses -= 1;
                                    self.unit_clauses.insert(clause_id);
                                } else if clause.len() == 2 {
                                    self.unit_clauses.remove(&clause_id);
                                }

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
                }
                _ => break,
            }
        }
    }

    /// Inference is possibly when there are some "Active" clauses
    /// and either pure literals or unit clauses.
    fn is_inference_possible(&self) -> bool {  // TODO: figure out this expression
        self.num_empty_clauses == 0
            && self.num_active_clauses > 0  // Not SAT
            // && (!self.pure_literals.is_empty() || !self.unit_clauses.is_empty())  // Something to infer
            && !self.unit_clauses.is_empty()
    }
    fn get_branch_variable(&self) -> (Variable, bool) {
        // TODO: either be able to implement one of these in hardware or add the lazy in
        match self.heuristic {
            SolverHeuristic::FirstVariable => (self.assignments.iter().position(|x| x.is_none()).unwrap() + 1, false),
            SolverHeuristic::MostLiteralOccurrences => self.get_most_literal_occurrences(),
            SolverHeuristic::MostVariableOccurrences => self.get_most_variable_occurrences(),
            SolverHeuristic::MinimizeClauseLength => {
                todo!("I got rid of this because it seemed infeasible")
            }
        }
    }

    fn branch_variable(&mut self, variable: Variable, value: bool) {
        self.assign_variable(variable, value);
    }
}

fn verify_assignment(expression: &Expression, assignment: &Assignment) -> bool {  
    expression.is_satisfied_by(assignment)
}

pub fn solve(expression: Expression, verify: bool) -> Option<Assignment> {
    let mut modifiable = expression.clone();
    // Old code would multithread another dpll with MinimizeClauseLength heuristic on clone of expression
    modifiable.optimize();
    modifiable.set_heuristic(SolverHeuristic::FirstVariable);

    let (solution, branches) = solve_dpll(&mut modifiable);
    if solution.is_some() && verify {
        let assignment = solution.clone().unwrap();
        if !verify_assignment(&expression, &assignment) {
            panic!("Solution is invalid!");
        }
    }

    solution
}

// Tests

pub fn main() {
    println!("the very beginning");
    let path = "/Users/tatestaples/Code/SatSwarm/tests/satlib/unsat/uuf250-01.cnf";
    let expression = parse_dimacs(path);
    println!("starting");
    println!("Active clauses: {}", expression.num_active_clauses);
    let start_time = std::time::Instant::now();
    let result = solve(expression, true);
    println!("Time: {}", start_time.elapsed().as_secs_f64());
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

    #[test]
    fn test_large_unsat_speed() {
        println!("the very beginning");
        let path = "/Users/tatestaples/Code/SatSwarm/tests/satlib/unsat/uuf200-099.cnf";
        let expression = parse_dimacs(path);
        println!("starting");
        println!("Active clauses: {}", expression.num_active_clauses);
        let start_time = std::time::Instant::now();
        let result = solve(expression, true);
        println!("Time: {}", start_time.elapsed().as_secs_f64());
    }
    
    #[test]
    fn test_weird_satlib() {
        let path = "/Users/tatestaples/Code/SatSwarm/tests/satlib/unsat/uuf200-098.cnf";
        parse_dimacs(path);
    }
}