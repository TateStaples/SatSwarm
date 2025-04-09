/*
This module defines the `Node` structure and its associated methods.

The `Node` structure represents a node in a network topology with dynamic neighbors.
Each neighbor can receive different types of messages for work distribution and coordination
in the SAT solver.

Message types: 
- Fork: Contains CNF assignment buffer state and variable assignments
- Success: Signal to broadcast SAT solution
*/


// use stp, fmt::Deug};
use std::fmt::Debug;
use rustsat::types::Var;

use crate::structures::clause_table::{Term, TermState};
use super::{clause_table::ClauseTable, message::{Message, MessageDestination, MessageQueue, TermUpdate, Watchdog}, util_types::{NodeId, VarId, CLAUSE_LENGTH, DEBUG_PRINT}};


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NodeState {  
    Busy,
    AwaitingFork,
    RecievingFork,
}

struct UnitPropagation {
    var_id: VarId,
    assignment: bool
}


pub struct Node {
    pub id: NodeId,                         // My id in the SatSwarm
    neighbors: Vec<NodeId>,             // NodeId of nodes that we can send fork messages to
    pub table: ClauseTable,                 // My understanding of the state
    state: NodeState,                   // What I am doing
    incoming_message: Option<Message>,  // message I am currently processing (should only ever be fork)
    watchdog: Watchdog,                 // watchdog to make sure we don't get stuck

    assignments: Vec<(VarId, bool, bool)>,   // What are the current assigned var & assignment & if speculative
    assigned: Vec<bool>,                     // Which variables are assigned
    lut_banks: u8
}

impl Node {
    // ----- initialization ----- //
    pub fn new(id: NodeId, table: ClauseTable) -> Self {
        let vars = table.num_vars;
        Node {
            id,                                                 // My id
            neighbors: Vec::new(),                              // NodeId of nodes that we can send fork messages to
            table,                                              // My understanding of the state
            state: NodeState::AwaitingFork,                     // make sure to start at false except for the first node so they don't repeat work
            incoming_message: None,                             // 
            watchdog: Watchdog::new(0, 10),
            assignments: Vec::new(),
            assigned: vec![false; vars as usize],  // Which variables are assigned
            lut_banks: 16,
        }
    }

    pub fn add_neighbor(&mut self, id: NodeId) {
        self.neighbors.push(id);
    }

    pub fn remove_neighbor(&mut self, id: NodeId) {
        self.neighbors.retain(|&n| n != id);
    }

    // ----- getters ----- //
    pub fn busy(&self) -> bool {return self.state != NodeState::AwaitingFork}
    pub fn activate(&mut self) {self.state = NodeState::Busy;}
    fn get_next_var(&self) -> Option<usize>{
        self.assigned.iter().position(|&x| !x)
    }

    // ----- clock update ----- //
    pub fn clock_update(&mut self, clock: u64, network: &mut MessageQueue, busy_nodes: &mut Vec<bool>) { 
        let msg = std::mem::replace(&mut self.incoming_message, None);
        match (&self.state, msg) {
            (NodeState::RecievingFork, Some(Message::Fork { assignments})) => {
                assert!(self.assignments.is_empty(), "Node {} received fork while still processing", self.id);
                self.watchdog.check(clock);
                for (var, assignment, is_speculative) in assignments {
                    self.substitute(var, assignment, false);  // FIXME: this is not cycle accurate as this parallelism should be dependent on banking
                }
                let var = self.get_next_var().expect("Forked SAT problem!") as VarId;
                self.substitute(var, true, false);
            },
            (NodeState::Busy, None) => {
                self.watchdog.check(clock);
                if DEBUG_PRINT {
                    println!("Assignments: {:?}", self.assignments);
                }
                if self.check_unsat() {
                    self.unsat();
                } else if let Some(UnitPropagation{var_id, assignment}) = self.find_unit_prop() {
                    self.substitute(var_id, assignment, false);
                } else {
                    self.branch(clock, network, busy_nodes);
                }
            },
            (NodeState::AwaitingFork, None) => {
                self.watchdog.check(clock);
            },  // do nothing, keep waiting
            (_, m) => panic!("{:?} received unexpected message {:?}", self, m)
        }
    }

    // ----- branching ----- //
    fn branch(&mut self, clock: u64, network: &mut MessageQueue, busy_nodes: &mut Vec<bool>) {
        if let Some(UnitPropagation{var_id, assignment}) = self.find_unit_prop() {
            assert!(!self.assignments.iter().any(|(v, _, _)| *v == var_id), "Node {} found unit prop for var {} that is already assigned", self.id, var_id);
            self.substitute(var_id, assignment, false);
            if DEBUG_PRINT {
                println!("Node {} unit propagating var {} to {}", self.id, var_id, assignment);
            }
        } else if let Some(var) = self.get_next_var() {
            // branching unknown variable
            let var = var as VarId;
            if let Some(neighbor_id) = self.neighbors.iter().find(|&&n| !busy_nodes[n as usize]) {
                if DEBUG_PRINT {
                    println!("Node {} branching to neighbor {}", self.id, neighbor_id);
                }
                // forked work
                busy_nodes[*neighbor_id as usize] = true;
                self.partner_branch(clock, network, var, *neighbor_id);
            } else {
                if DEBUG_PRINT {
                    println!("Node {} speculating on {}", self.id, var);
                }
                // speculative work
                self.speculative_branch(var);
            }
        } else  {
            if DEBUG_PRINT {
                println!("Node {} is SAT", self.id);
            }
            // we are done done because there is no more work
            self.sat(clock, network);
        }
    }

    fn partner_branch(&mut self, clock: u64, network: &mut MessageQueue, var: VarId, neighbor_id: NodeId) {
        assert!(self.state == NodeState::Busy, "Node {} is not in busy state", self.id);
        
        // copy the CNF state and send the fork. Then continue with the other branch 
        let fork_msg = Message::Fork { assignments: self.assignments.clone() };  // FIXME: this has the wrong data
        self.send_message(clock, network, MessageDestination::Neighbor(neighbor_id), fork_msg);  

        // now substitute the variable here
        self.substitute(var, false, false);  // not speculative because partner has the other work
    }

    fn speculative_branch(&mut self, var: VarId) {
        assert!(self.state == NodeState::Busy, "Node {} is not in branching state", self.id);
        self.substitute(var, false, true);
    }

    // ----- processing ----- //
    fn find_unit_prop(&self) -> Option<UnitPropagation> {  // implemented with a priority encoder
        for clause in self.table.clause_table.iter() {
            if clause.iter().filter(|(_, state)| *state == TermState::Symbolic).count() == 1 && !clause.iter().any(|(_, state)| *state == TermState::True) {
                let (term, term_state) = clause.iter().find(|(_, state)| *state == TermState::Symbolic).unwrap();
                assert!(*term_state == TermState::Symbolic, "Term is not symbolic");
                return Some(UnitPropagation{var_id: term.var, assignment: !term.negated});
            }
        }
        None
    }
    fn substitute(&mut self, var: VarId, assignment: bool, speculative: bool) {
        assert!(self.state == NodeState::Busy || self.state == NodeState::RecievingFork, "Node {} is not in branching state", self.id);
        self.state = NodeState::Busy;
        self.assignments.push((var, assignment, speculative));
        self.assigned[var as usize] = true;
        // let Self { table }
        for clause in self.table.clause_table.iter_mut() {
            for (term, term_state) in clause.iter_mut() {
                if term.var == var {
                    let state = if assignment != term.negated {TermState::True} else {TermState::False};
                    *term_state = state;
                }
            }
        }
    }
    fn reset_terms(&mut self, var: VarId) {
        assert!(!self.assignments.iter().any(|(v, _, _)| *v == var), "Node {} tried to reset var {} that is still assigned", self.id, var);
        assert!(self.assigned[var as usize], "Node {} tried to reset var {} that is not assigned", self.id, var);
        self.assigned[var as usize] = false;
        self.table.clause_table.iter_mut().for_each(|clause| {
            for (term, term_state) in clause.iter_mut() {
                if term.var == var {
                    *term_state = TermState::Symbolic; // Reset the term state
                }
            }
        });
    }
    fn check_unsat(&self) -> bool {
        self.table.clause_table.iter().any(|clause| {
            clause.iter().all(|(term, term_state)| *term_state == TermState::False)
        })
    }

    // ----- termination ----- //
    fn clear_state(&mut self) {
        self.assignments.clear();
        if DEBUG_PRINT {
            println!("Node {} clearing state", self.id);
        }
        self.state = NodeState::AwaitingFork; 
    }
    fn unsat(&mut self) {
        if DEBUG_PRINT {
            println!("Node {} is UNSAT", self.id);
        }
        if !self.assignments.iter().any(|(_, _, is_speculative)| *is_speculative) { 
            self.clear_state();
        } else {
            self.backtrack();
        }
    }

    fn backtrack(&mut self) {
        assert!(self.state == NodeState::Busy, "Node {} is not in branching state", self.id);
        if DEBUG_PRINT {
            println!("Node {} backtracking", self.id);
        }
        loop {
            let (var, assignment, is_speculative) = self.assignments.pop().expect("Node backtracing with no speculative state");
            assert!(self.assigned[var as usize], "Node {} tried to backtrack var {} that is not assigned", self.id, var);
            if is_speculative {
                // FIXME: this probably shouldn't be implemented in a single clock cycle. Make this a separate node state
                self.substitute(var, !assignment, false);
                break; 
            } else {
                self.reset_terms(var);
            }
        }
    }

    fn sat(&mut self, clock: u64, network: &mut MessageQueue) {
        println!("Node {} is SAT", self.id);
        // self.state = NodeState::AwaitingFork;
        self.send_message(clock, network, MessageDestination::Broadcast, Message::Success);
    }

    // ----- Networking interface ----- //
    pub fn recieve_message(&mut self, from: MessageDestination, message: Message) {
        match from {
            MessageDestination::Neighbor(id) => {
                assert!(self.neighbors.contains(&id), "Node {:?} received message from non-neighbor", self);
            },
            _ => panic!("{:?} received unexpected message source", self)
        }
        assert!(self.incoming_message.is_none(), "Node received multiple messages in one cycle");
        self.incoming_message = Some(message);
        if self.state == NodeState::AwaitingFork {
            match self.incoming_message {
                Some(Message::Fork {..}) => {
                    self.state = NodeState::RecievingFork;
                },
                _ => {
                    panic!("Node {} received unexpected message", self.id);
                }
            }
            self.state = NodeState::RecievingFork;
        }
    }

    fn send_message(&self, clock: u64, network: &mut MessageQueue, dest: MessageDestination, message: Message) {
        if DEBUG_PRINT {
            println!("Node {} sending message {:?} to {:?}", self.id, message, dest);
        }
        network.start_message(clock, MessageDestination::Neighbor(self.id), dest, message);
    }
} 
impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node id: {}, state: {:?}, neighbors: {:?}", self.id, self.state, self.neighbors)
    }
    
}