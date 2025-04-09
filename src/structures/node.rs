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
use std::{collections::HashMap, fmt::Debug};
use crate::structures::clause_table::{Term, TermState};
use super::{clause_table::ClauseTable, message::{Message, MessageDestination, MessageQueue, TermUpdate, Watchdog}, util_types::{NodeId, VarId, CLAUSE_LENGTH}};


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NodeState {  
    Busy,
    AwaitingFork,
    RecievingFork,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpeculativeDepth { 
    Depth(VarId), // speculative depth 0 = guarenteed, 1 = speculative, 2 = further speculation
    Unassigned
}
struct VarUpdate {
    var_id: VarId,                                  // which variable are we updating
    clause_index: usize,                            // which clause we are updating in
    assignment: bool,                               // what is the assignment (true/false)
    reset: bool,                                    // should we reset the variables of higher depth
    // speculative: bool,                              // is this a speculative assignment
    depth: VarId,                                   // what is the depth of the assignment
}
struct UnitPropagation {
    speculative_depth: VarId,
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

    parallel_clauses: usize,                    // how many clauses are checked per clock cycle
    pipeline_size: usize,                       // how many pipeline stages are available at a given time
    var_updates: Vec<VarUpdate>,                // what variables have been assigned and what their state is
    
    // This is our backtracking state and the details are not finalized yet
    update: Vec<SpeculativeDepth>,              // When each variable has been assigned
    speculative_branches: Vec<VarId>,           // What are previous speculative assignments
    unit_propagation: Vec<UnitPropagation>,     // What are previous unit propagation assignments
    
    
}
// TODO: update SAT to be when all variables are set (this should be a rare case)
impl Node {
    // ----- initialization ----- //
    pub fn new(id: NodeId, table: ClauseTable) -> Self {
        let vars = table.num_vars;
        Node {
            id,                                                 // My id
            neighbors: Vec::new(),                              // NodeId of nodes that we can send fork messages to
            table,                                              // My understanding of the state
            update: vec![SpeculativeDepth::Unassigned; vars],   // At what speculative depth was each variable assigned (0=unassigned)
            var_updates: Vec::new(),                            // Which clause are we currently processing
            parallel_clauses: 1,                                  // How many clauses are checked per clock cycle
            speculative_branches: Vec::new(),                   // What is the speculative of newly assigned variables (some optimizaiton to use this as both a speculative and unit propagation buffer)
            state: NodeState::AwaitingFork,                     // make sure to start at false except for the first node so they don't repeat work
            incoming_message: None,                             // 
            watchdog: Watchdog::new(0, 500),
            pipeline_size: 10,
            unit_propagation: Vec::new(),
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
    pub fn activate(&mut self) {self.state = NodeState::Busy}
    fn get_next_var(&self) -> Option<usize>{
        return self.update.iter().position(|x| *x == SpeculativeDepth::Unassigned) // For now get the index of the first unassigned variable
    }
    fn get_deepest_speculation(&self) -> VarId {
        let mut max = 0;
        for var in self.update.iter() {
            match var {
                SpeculativeDepth::Depth(depth) => {
                    if *depth > max {
                        max = *depth;
                    }
                },
                _ => {}
            }
        }
        return max;
    }
    // ----- clock update ----- //
    pub fn clock_update(&mut self, clock: u64, network: &mut MessageQueue, busy_nodes: &mut Vec<bool>) { 
        let msg = std::mem::replace(&mut self.incoming_message, None);
        match (&self.state, msg) {
            (NodeState::RecievingFork, Some(Message::Fork {table, assigned_vars})) => {
                self.watchdog.check(clock);
                assert!(self.speculative_branches.is_empty(), "Node {} received fork while still processing", self.id);
                self.table = table;
                assert!(self.update.len() == assigned_vars.len(), "nodes have different number of variables");
                self.update = assigned_vars;
                let var = self.get_next_var().expect("Forked SAT problem!") as VarId;
                self.substitute(var, true, false, self.get_deepest_speculation()+1);
            },
            (NodeState::Busy, None) => {
                if self.update.len() < self.pipeline_size {
                    self.branch(clock, network, busy_nodes);
                }
                let Self {   // Doing bs to avoid borrowing issues
                    table, 
                    var_updates, 
                    update, 
                    unit_propagation ,
                    ..
                } = self;
                let mut unsat_depth = None;
                for var_update in var_updates.iter_mut() {
                    for _ in 0..self.parallel_clauses {
                        let success = Self::process_clause(table, var_update, update, unit_propagation);
                        var_update.clause_index += 1;
                        if !success {
                            unsat_depth = Some(var_update.depth);
                            break;
                        }
                        if var_update.clause_index >= table.num_clauses || self.state != NodeState::Busy {
                            break;
                        }
                    }
                    self.watchdog.check(clock);
                }
                if let Some(depth) = unsat_depth {
                    self.unsat(depth);  // finally can make mutable calls here
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
        if let Some(UnitPropagation{var_id, assignment, speculative_depth}) = self.unit_propagation.pop() {
            // unit propagation
            self.substitute(var_id, assignment, false, speculative_depth);
        } else if let Some(var) = self.get_next_var() {
            // branching unknown variable
            let var = var as VarId;
            if let Some(neighbor_id) = self.neighbors.iter().find(|&&n| busy_nodes[n as usize]) {
                // forked work
                busy_nodes[self.id as usize] = true;
                self.partner_branch(clock, network, var, *neighbor_id);
            } else {
                // speculative work
                self.speculative_branch(var);
            }
        } else if self.update.is_empty() {
            // we are done done because there is no more work
            // TODO: check that the unsat substitutes fast enough
            self.sat(clock, network);
        }
    }

    fn partner_branch(&mut self, clock: u64, network: &mut MessageQueue, var: VarId, neighbor_id: NodeId) {
        assert!(self.state == NodeState::Busy, "Node {} is not in busy state", self.id);
        
        // copy the CNF state and send the fork. Then continue with the other branch 
        let fork_msg = Message::Fork {table: self.table.clone(), assigned_vars: self.update.clone()};
        self.send_message(clock, network, MessageDestination::Neighbor(neighbor_id), fork_msg);  

        // now substitute the variable here
        self.substitute(var, false, false, self.get_deepest_speculation()+1);
    }

    fn speculative_branch(&mut self, var: VarId) {
        assert!(self.state == NodeState::Busy, "Node {} is not in branching state", self.id);
        self.speculative_branches.push(var);  //  I think this can be removedd
        self.substitute(var, false, false, self.get_deepest_speculation()+1);
    }

    // ----- processing ----- //
    fn substitute(&mut self, var: VarId, assignment: bool, reset: bool, speculative_depth: VarId) {
        assert!(self.state == NodeState::Busy || self.state == NodeState::RecievingFork, "Node {} is not in branching state", self.id);
        self.state = NodeState::Busy;
        self.var_updates.push(VarUpdate {
            var_id: var,                    // which variable are we updating
            clause_index: 0,                // start at the beginning
            assignment,                     // what is the assignment (true/false) 
            reset,                          // should we reset the variables of higher depth
            // speculative: false,             
            depth: speculative_depth,         // what is the depth of the assignment
        });
    }
    
    fn mask(table: &ClauseTable, update_buffer: &mut Vec<SpeculativeDepth>, var_update: &VarUpdate) -> [TermUpdate; CLAUSE_LENGTH] {
        let mut iter = table.clause_table[var_update.clause_index].iter().map(|(Term { var, negated }, _)| {
            let speculative_depth = match update_buffer[*var as usize] {
                SpeculativeDepth::Depth(depth) => depth,
                SpeculativeDepth::Unassigned => 0,
            };
            if *var == var_update.var_id {
                if *negated == !var_update.assignment {
                    TermUpdate::True
                } else {
                    TermUpdate::False
                }
            } else if speculative_depth >= var_update.depth {
                TermUpdate::Reset
            } else {
                TermUpdate::Unchanged
            }
        });

        [
            iter.next().expect("Iterator did not yield enough elements"),
            iter.next().expect("Iterator did not yield enough elements"),
            iter.next().expect("Iterator did not yield enough elements"),
        ]
    }

    fn process_clause(clause_table: &mut ClauseTable, var_update: &VarUpdate, update_buffer: &mut Vec<SpeculativeDepth>, unit_props: &mut Vec<UnitPropagation>) -> bool {
        assert!(var_update.clause_index < clause_table.clause_table.len(), "reading past the end of the clause");
        // later optimizations mean we can fast forward through tautologies
        let mask = Self::mask(clause_table, update_buffer, var_update);
        let current_clause = &mut clause_table.clause_table[var_update.clause_index];

        // assign the variable
        let mut _symbolic_count = 0; //  potentially useful for later optimizations (unit propagation)
        for ((_, term), result) in current_clause.iter_mut().zip(mask) {
            _symbolic_count += if *term == TermState::Symbolic {1} else {0};
            match result {
                TermUpdate::True => { // true in clause makes the whole clause true
                    update_buffer[var_update.var_id as usize] = SpeculativeDepth::Depth(var_update.depth);
                    *term = TermState::True;
                },
                TermUpdate::False => {
                    update_buffer[var_update.var_id as usize] = SpeculativeDepth::Depth(var_update.depth);
                    *term = TermState::False;
                },
                TermUpdate::Reset => {
                    update_buffer[var_update.var_id as usize] = SpeculativeDepth::Unassigned;
                    *term = TermState::Symbolic;
                },
                TermUpdate::Unchanged => {}
            }
        }
        if current_clause.iter().all(|(_, state)| *state == TermState::False) {
            // self.unsat(var_update.depth);
            return false;
        } else if _symbolic_count == 1 {
            todo!("unit propagation");
        }
        return true;
    }

    // ----- termination ----- //
    fn unsat(&mut self, speculative_depth: VarId) {
        self.var_updates.retain(|var_update| var_update.depth < speculative_depth);
        if self.speculative_branches.is_empty() {
            self.state = NodeState::AwaitingFork; 
        } else {
            self.backtrack();
        }
    }

    fn backtrack(&mut self) {
        let var = self.speculative_branches.pop().expect("No branches to backtrack");
        let current_depth = if let Some(spec) = self.speculative_branches.last() {
            if let SpeculativeDepth::Depth(depth) = self.update[*spec as usize] {
                depth + 1
            } else {
                panic!("Speculating on unassigned variable");
            }
        } else {
            0
        };
        self.substitute(var, true, true,  current_depth);
    }

    fn sat(&mut self, clock: u64, network: &mut MessageQueue) {
        println!("Node {} is SAT", self.id);
        self.state = NodeState::AwaitingFork;
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
        network.start_message(clock, MessageDestination::Neighbor(self.id), dest, message);
    }
} 
impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node id: {}, state: {:?}, neighbors: {:?}", self.id, self.state, self.neighbors)
    }
    
}