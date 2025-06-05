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
use std::process::exit;
use rustsat::types::Var;
use crate::structures::clause_table::{ClauseIdx, ProblemState, TermState};
use crate::structures::util_types::Time;
use super::{clause_table::ClauseTable, util_types::{NodeId, VarId, CLAUSE_LENGTH, DEBUG_PRINT}};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeState {
    /// Making decision
    Busy, 
    /// Awaiting fork
    Idle,
    /// DONE. Has solved the problem!
    SAT
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VariableAssignment {
    pub var_id: VarId,
    pub assignment: bool,
    pub time: Time,
    pub cause: AssignmentCause
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fork {
    pub variable_assignments: Vec<Option<bool>>,  // TODO: either copy state or edit from diff in variable assignments (future optimization)
    pub fork_time: Time,
}

/// Node class defn
pub struct Node {
    /// Unique identifier for the node.
    pub id: NodeId,
    pub state: NodeState,
    /// Local understanding of the SAT problem state.
    pub table: ClauseTable,
    /// History of variable assignments.
    pub assignment_history: Vec<VariableAssignment>,
    /// What variables (indexed by VarId) are assigned and to what
    // TODO: should probably change TermState to also be Option<bool>
    pub assignments: Vec<Option<bool>>,
    /// Tracks unit propagation assignments.
    unit_propagation: Vec<VariableAssignment>,
    /// The last updated clock cycle for the node.
    pub local_time: Time,
    // ----- configs ----- // 
    /// Number of clauses checked per clock cycle.
    parallel_clauses: usize,
    /// Number of pipeline stages available at a given time.
    cycles_per_eval: usize,
}


impl Node {
    pub fn new(id: NodeId, table: ClauseTable, parallel_clauses: usize, cycles_per_eval: usize) -> Self {
        let vars = table.number_of_vars();
        // println!("Configs: parallel_clauses: {}, cycles_per_eval: {}", parallel_clauses, cycles_per_eval);
        Node {
            id,
            state: NodeState::Idle,
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
    fn variable_decision(&self) -> Option<usize> {
        self.assignments.iter().position(|x| x.is_none())
    }

    // ----- run ----- //
    /// The call to the first one. Doesn't start in backtracking or idle mode
    pub fn activate(&mut self) {
        self.branch();
    }

    /// Called by the node upon UNSAT with a certain >0 depth. 
    /// Undoes speculative assignment and clears its implications
    pub fn retry(&mut self) {
        if DEBUG_PRINT {
            println!("Node {} is retrying", self.id);
        }

        // clear future unit props because they are irrelevant now due to UNSAT
        self.unit_propagation.clear();

        // go through all previous assignments chronologically
        // if speculative assignment then rollback
        while let Some(assignment) = self.assignment_history.pop() {
            let VariableAssignment { var_id, assignment, cause, .. } = assignment;
            match cause {
                AssignmentCause::Speculative => {
                    // TODO: maybe add another cause (no performance gain but interp)
                    self.substitute(var_id, !assignment, AssignmentCause::Fork);  
                    self.branch();
                    break;
                } 
                // (shaan): confused about this one
                _ => { self.reset(var_id, TermState::Symbolic); }
            }
        }

        // idle if there is nothing to backtrack to
        if self.assignment_history.is_empty() {
            self.state = NodeState::Idle;
        }
    }


    /// On receiving a fork
    /// Update the assignments table
    /// add time to receive a fork
    pub fn receive_fork(&mut self, fork: Fork) {
        if DEBUG_PRINT {
            println!("Node {:?} is receiving fork {:?}", self, fork);
        }
        let Fork {
            variable_assignments,
            fork_time
        } = fork;

        // lovely minimizing writes
        let changes: Vec<_> = self.assignments.iter().zip(variable_assignments.iter())
            .enumerate()
            .filter(|&(_, (mine, new))| mine != new)
            .map(|(idx, (_, new))| (idx as VarId, new))
            .collect();

        for (var_id, assignment) in changes {
            match assignment {
                Some(true) => self.reset(var_id, TermState::True),
                Some(false) => self.reset(var_id, TermState::False),
                None => self.reset(var_id, TermState::Symbolic),
            }
        }
        assert!(self.assignments == variable_assignments);
        self.assignment_history.clear();
        self.unit_propagation.clear();
        self.local_time = fork_time;
        if !self.problem_unsat() {
            self.branch();
        }
    }

    // ----- branching ----- //
    //TODO (shaan): realistic delays for branching + explanation why
    /// branching on a node to new assignment
    /// (1) if unit prop possible, unit prop
    /// (2) if no unit prop, then choose next var to assign
    /// (3) if neither then is SAT (bc unsat is checked for)
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
                if DEBUG_PRINT {
                    println!("Node {:?} is unit propagating {} for var {}", self, assignment, var_id);
                }
                self.local_time +=1;
                if self.substitute(var_id, assignment, AssignmentCause::UnitPropagation) {
                    // UNSAT
                    break
                }
            } else if let Some(var) = self.variable_decision() {
                if DEBUG_PRINT {
                    println!("Node {:?} is speculatively branching on var {}", self, var);
                }
                self.local_time +=1;
                // branching unknown variable
                let var = var as VarId;
                self.substitute(var, false, AssignmentCause::Speculative);
            } else {
                self.local_time += 1;
                if DEBUG_PRINT {
                    println!("Node {} is SAT", self.id);
                }
                self.state = NodeState::SAT;
                break
            }
        }
    }


    // ----- processing ----- //

    /// Instant memory/assignment updates for modelling purposes.
    /// Used to model fast, and where local_time is updated in parallel/not on critical path
    fn reset(&mut self, var: VarId, value: TermState) {
        let lookup = &self.table.transpose[var as usize];
        let negated = match value {
            TermState::True => TermState::False,
            TermState::False => TermState::True,
            TermState::Symbolic => TermState::Symbolic,
        };
        
        self.assignments[var as usize] = match value {
            TermState::True => Some(true),
            TermState::False => Some(false),
            TermState::Symbolic => None,
        };
        for (clause_idx, term_idx) in lookup.pos.iter() {
            self.table.problem_state[*clause_idx][*term_idx] = value;
        }
        for (clause_idx, term_idx) in lookup.neg.iter() {
            self.table.problem_state[*clause_idx][*term_idx] = negated;
        }
    }


    /// Local node decides to subsitute new variable (ie new assignment)
    /// assigns new assignments into memory
    /// also calculates delays for going through memory
    fn substitute(&mut self, var: VarId, assignment: bool, cause: AssignmentCause) -> bool {
        let before = self.local_time;
        // add new variable assignment to the assignment history
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
            table, unit_propagation,
            local_time, parallel_clauses, cycles_per_eval, ..
        } = self;
        let lookup = &table.transpose[var as usize];

        // update assignments in clause table
        // also add local time delays via reach_time for unsat, 
        // and if not unsat, then add delay via reach_time total
        if assignment == true {
            for (clause_idx, term_idx) in lookup.pos.iter() {
                table.problem_state[*clause_idx][*term_idx] = TermState::True;
            }
            for (clause_idx, term_idx) in lookup.neg.iter() {
                table.problem_state[*clause_idx][*term_idx] = TermState::False;
                if Self::clause_unsat(table, *clause_idx, unit_propagation) {
                    // UNSAT!
                    *local_time += Self::reach_time(*clause_idx, *parallel_clauses, *cycles_per_eval);
                    return true;
                }
            }
        } else {
            for (clause_idx, term_idx) in lookup.pos.iter() {
                table.problem_state[*clause_idx][*term_idx] = TermState::False;
                if Self::clause_unsat(table, *clause_idx, unit_propagation) {
                    // UNSAT!
                    *local_time += Self::reach_time(*clause_idx, *parallel_clauses, *cycles_per_eval);
                    return true;
                }
            }
            for (clause_idx, term_idx) in lookup.neg.iter() {
                table.problem_state[*clause_idx][*term_idx] = TermState::True;
            }
        }
        // No fault
        *local_time += Self::reach_time(table.number_of_clauses(), *parallel_clauses, *cycles_per_eval);
        assert!(*local_time > before);
        false
    }


    /// Determining if clause is unsat
    /// also checks if unit propagation is possible, and adds variable and assignment onto unit prop stack
    fn clause_unsat(table: &ClauseTable, clause_idx: usize, unit_propagation: &mut Vec<VariableAssignment>) -> bool {
        let current_clause = &table.problem_state[clause_idx];
        let count = current_clause.iter().filter(|&state| *state == TermState::Symbolic).count();
        if count == 0 && current_clause.iter().all(|state| *state == TermState::False) {
            // UNSAT
            return true;
        } else if count == 1 && !current_clause.iter().any(|state| *state == TermState::True) {
            // unit propagation
            let term_idx = current_clause.iter().position(|state| *state == TermState::Symbolic).unwrap();
            let symbol = &table.symbolic_table[clause_idx][term_idx];
            unit_propagation.push(
                VariableAssignment {
                    var_id: symbol.var,
                    assignment: !symbol.negated,
                    time: 0,
                    cause: AssignmentCause::UnitPropagation,
                }
            );
        }
        false
    }

    /// checks if problem is UNSAT or not
    /// if UNSAT, adds time until node is freed
    /// if undecided, adds time to go through all clauses in memory system to local time

    // TODO (shaan): check if local time addition is correct
    // probably need to change cycles per eval
    pub fn problem_unsat(&mut self) -> bool {
        let Self {
            table, unit_propagation,
            parallel_clauses, cycles_per_eval, local_time, ..
        } = self;
        for clause_idx in 0..table.number_of_clauses() {
            if Self::clause_unsat(table, clause_idx, unit_propagation) {
                *local_time += Self::reach_time(clause_idx, *parallel_clauses, *cycles_per_eval);
                return true;
            }
        }
        *local_time += Self::reach_time(table.number_of_clauses(), *parallel_clauses, *cycles_per_eval);
        false
    }

    /// Time to reach a certain clauses in memory
    fn reach_time(clause_idx: usize, parallel_clauses: usize, cycles_per_eval: usize) -> Time {
        (Self::div_up(clause_idx, parallel_clauses) * cycles_per_eval) as Time
    }

    /// Ceiling division
    fn div_up(a: usize, b: usize) -> usize { (a + (b - 1)) / b }
    

    /// prints state of solution
    pub fn print_model(&self) {
        for clause_idx in 0..self.table.number_of_clauses() {
            for term_idx in 0..CLAUSE_LENGTH {
                let symbol = self.table.symbolic_table[clause_idx][term_idx];
                let value = self.table.problem_state[clause_idx][term_idx];
                let assigned_value = self.assignments[symbol.var as usize];
                match assigned_value { 
                    Some(a) => if a == symbol.negated { assert_eq!(value, TermState::False) } else { assert_eq!(value, TermState::True) },
                    None => {}
                }
                if symbol.negated {print!("¬")}
                let value_char = match value { 
                    TermState::True => 'T',
                    TermState::False => 'F',
                    TermState::Symbolic => '?',
                };
                print!("{}({})\t", symbol.var, value_char);
            }
            println!();
        }
    }
    
    /// State to optional type
    fn term_state_to_option(state: TermState) -> Option<bool> {
        match state { 
            TermState::True => Some(true),
            TermState::False => Some(false),
            TermState::Symbolic => None,
        }
    }
}
impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(id: {} @ time: {})", self.id, self.local_time)
    }
    
}