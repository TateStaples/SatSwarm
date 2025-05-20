/*
This module defines the `Node` structure and its associated methods.

The `Node` structure represents a node in a network topology with dynamic neighbors.
Each neighbor can receive different types of messages for work distribution and coordination
in the SAT solver.

Message types: 
- Fork: Contains CNF assignment buffer state and variable assignments
- Success: Signal to broadcast SAT solution
*/
use std::collections::{HashSet, VecDeque};
// use stp, fmt::Deug};
use std::fmt::Debug;
use rustsat::types::Var;
use crate::structures::clause_table::{ProblemState, TermState};
use super::{clause_table::ClauseTable, util_types::{NodeId, VarId, CLAUSE_LENGTH, DEBUG_PRINT}};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeState {
    /// Making decision
    Busy, 
    /// Awaiting fork
    Idle,
    /// Awaiting fork with no active neighbors
    Sleep,
    /// DONE. Has solved the problem!
    SAT
}
/// How deep into the problem this variable is assigned
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpeculativeDepth { 
    /// The variable is assigned at a certain depth with a certain assignment
    Depth(VarId, bool),
    /// The variable is unassigned
    Unassigned
}

/// Label of how the value of a variable was assigned
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AssignmentCause {
    /// Took one branch and sent the other branch to a neighbor
    Fork,
    /// Assignment forced by unit propagation
    UnitPropagation,
    /// Assignment was guessed and might not be correct
    Speculative,
}
pub struct VariableAssignment {
    pub var_id: VarId,
    pub assignment: bool,
    pub time: u64,
    pub cause: AssignmentCause
}

pub struct Fork {
    variable_assignments: Vec<Option<bool>>,  // TODO: either copy state or edit from diff in variable assignments (future optimization)
    fork_time: u64,
}
pub struct Node {
    /// Unique identifier for the node.
    pub id: NodeId,
    pub state: NodeState,
    /// Local understanding of the SAT problem state.
    pub table: ClauseTable,
    /// History of variable assignments.
    pub assignment_history: Vec<VariableAssignment>,
    pub assignments: Vec<Option<bool>>,
    /// Tracks unit propagation assignments.
    unit_propagation: Vec<VariableAssignment>,
    /// The last updated clock cycle for the node.
    pub local_time: u64,

    // ----- configs ----- // 
    /// Number of clauses checked per clock cycle.
    parallel_clauses: usize,
    /// Number of pipeline stages available at a given time.
    cycles_per_eval: usize,
}


// TODO: update SAT to be when all variables are set (this should be a rare case)
impl Node {
    
    /// Creates a new node with given arguments
    // FIXME: this constructor need to be updated
    pub fn new(id: NodeId, table: ClauseTable, parallel_clauses: usize, cycles_per_eval: usize) -> Self {
        let vars = table.number_of_vars();
        Node {
            id,
            state: NodeState::Sleep,
            table,
            assignment_history: vec![],
            assignments: vec![None; vars],
            unit_propagation: vec![],
            local_time: 0,
            parallel_clauses,
            cycles_per_eval,
        }
    }
    

    
    /// Next speculative variable to decide
    /// Currently using the first (phonetically) unassigned variable
    /// TODO: Correct to Shaan's first by appearance unassigned variable
    fn variable_decision(&self) -> Option<usize>{
        self.assignments.iter().position(|x| x.is_none())
    }


    // ----- run ----- //
    pub fn activate(&mut self) {
        self.branch();
    }
    /// Called by the node upon UNSAT with a certain >0 depth. Undoes speculative assignment and clears its implications
    pub fn retry(&mut self) {
        // TODO: backtrack should take some time (sounds like 1 pass over the clauses)
        self.unit_propagation.clear();
        while let Some(assignment) = self.assignment_history.pop() {
            let VariableAssignment { var_id, assignment, cause, .. } = assignment;
            match cause {
                AssignmentCause::Speculative => {
                    self.substitute(var_id, !assignment, AssignmentCause::Fork);  // TODO: maybe add another cause (no performance gain but interp)
                    self.branch();
                    break;
                }
                _ => { self.reset(var_id); }
            }
        }
        if self.assignment_history.is_empty() {
            self.state = NodeState::Idle;
        }
    }
    pub fn receive_fork(&mut self, fork: Fork) {
        let Fork {
            variable_assignments,
            fork_time
        } = fork;
        for (idx, (mine, new)) in self.assignments.iter().zip(variable_assignments.iter()).enumerate() {
            if mine != new {
                let idx = idx as VarId;
                if let Some(assignment) = new {
                    self.substitute(idx, *assignment, AssignmentCause::Fork);
                }
            }
        }
        self.assignment_history.clear();
        self.unit_propagation.clear();
        self.local_time = fork_time;
    }

    // ----- branching ----- //
    fn branch(&mut self) {
        self.state = NodeState::Busy;
        loop {
            if let Some(assignment) = self.unit_propagation.pop() {
                let VariableAssignment { var_id, assignment, .. } = assignment;
                if self.assignments[var_id as usize].is_some() {
                    if self.assignments[var_id as usize].unwrap() == assignment { continue; }
                    // self.unsat();
                    break;
                }
                if self.substitute(var_id, assignment, AssignmentCause::UnitPropagation) {
                    // UNSAT
                }
            } else if let Some(var) = self.variable_decision() {
                // branching unknown variable
                let var = var as VarId;
               
            } else {
                if DEBUG_PRINT {
                    println!("Node {} is SAT", self.id);
                }
                self.state = NodeState::SAT;
                break
            }
        }
    }


    // ----- processing ----- //
    fn reset(&mut self, var: VarId) {
        // TODO: discuss with Shaan what this looks like in hardware and how much time it takes
        let lookup = &self.table.transpose[var as usize];
        self.assignments[var as usize] = None;
        for (clause_idx, term_idx) in lookup.pos.iter() {
            self.table.problem_state[*clause_idx][*term_idx] = TermState::Symbolic;
        }
        for (clause_idx, term_idx) in lookup.neg.iter() {
            self.table.problem_state[*clause_idx][*term_idx] = TermState::Symbolic;
        }
    }
    fn substitute(&mut self, var: VarId, assignment: bool, cause: AssignmentCause) -> bool {
        // FIXME: where should time updating be handled - probably here
        self.assignment_history.push(
            VariableAssignment {
                var_id: var,
                assignment,
                time: self.local_time,
                cause,
            }
        );
        self.assignments[var as usize] = Some(assignment);

        let Self {
            table, unit_propagation, ..
        } = self;
        let lookup = &table.transpose[var as usize];
        if assignment == true {
            for (clause_idx, term_idx) in lookup.pos.iter() {
                table.problem_state[*clause_idx][*term_idx] = TermState::True;
            }
            for (clause_idx, term_idx) in lookup.neg.iter() {
                table.problem_state[*clause_idx][*term_idx] = TermState::False;
                if !Self::check_clause(table, *clause_idx, unit_propagation) {
                    // UNSAT!
                    return true;
                }
            }
        } else {
            for (clause_idx, term_idx) in lookup.pos.iter() {
                table.problem_state[*clause_idx][*term_idx] = TermState::False;
                if !Self::check_clause(table, *clause_idx, unit_propagation) {
                    // UNSAT!
                    return true;
                }
            }
            for (clause_idx, term_idx) in lookup.neg.iter() {
                table.problem_state[*clause_idx][*term_idx] = TermState::True;
            }
        }
        // No fault
        false
    }

    fn check_clause(table: &ClauseTable, clause_idx: usize, unit_propagation: &mut Vec<VariableAssignment>) -> bool {
        let current_clause = &table.problem_state[clause_idx];
        let count = current_clause.iter().filter(|&state| *state == TermState::Symbolic).count();
        if count == 0 && current_clause.iter().all(|state| *state == TermState::False) {
            // UNSAT
            return false;
        } else if count == 1 && !current_clause.iter().any(|state| *state == TermState::True){
            // unit propagation
            let term_idx = current_clause.iter().position(|state| *state == TermState::Symbolic).unwrap();
            let symbol = &table.symbolic_table[clause_idx][term_idx];
            unit_propagation.push(
                VariableAssignment {
                    var_id: term_idx as VarId,
                    assignment: !symbol.negated,
                    time: 0,
                    cause: AssignmentCause::UnitPropagation,
                }
            );
        }
        true
    }

} 
impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node id: {}, state: {:?}, assignments: {:?}", self.id, self.state, self.assignments)
    }
    
}