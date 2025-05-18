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
use crate::structures::clause_table::{Term, TermState};
use super::{clause_table::ClauseTable, message::{Message, MessageDestination, MessageQueue, TermUpdate, Watchdog}, util_types::{NodeId, VarId, CLAUSE_LENGTH, DEBUG_PRINT}};


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NodeState {  
    Busy,
    AwaitingFork,
    RecievingFork,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpeculativeDepth { 
    Depth(VarId, bool), // speculative depth 0 = guarenteed, 1 = speculative, 2 = further speculation
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
    /// Unique identifier for the node.
    pub id: NodeId,
    /// List of neighboring nodes that can send fork messages.
    neighbors: Vec<NodeId>,
    /// Local understanding of the SAT problem state.
    pub table: ClauseTable,
    /// Current state of the node.
    state: NodeState,

    /// Message currently being processed.
    incoming_message: Option<Message>,
    /// Watchdog to prevent node from getting stuck.
    watchdog: Watchdog,
    /// Number of clauses checked per clock cycle.
    parallel_clauses: usize,
    /// Number of pipeline stages available at a given time.
    pipeline_size: usize,
    /// Variables that have been assigned and their state.
    var_updates: Vec<VarUpdate>,

    /// Tracks when each variable was assigned in the SAT solving process.
    /// Each element corresponds to a variable and contains its assignment depth and value.
    assignment_time: Vec<SpeculativeDepth>,
    /// Tracks the speculative branches of newly assigned variables.
    speculative_branches: Vec<VarId>,
    /// Tracks unit propagation assignments.
    unit_propagation: Vec<UnitPropagation>,
}


// TODO: update SAT to be when all variables are set (this should be a rare case)
impl Node {
    
    /// Creates a new node with given arguments
    pub fn new(id: NodeId, table: ClauseTable, parallel_clauses: usize) -> Self {
        let vars = table.num_vars;
        Node {
            id,                                                 // My id
            neighbors: Vec::new(),                              // NodeId of nodes that we can send fork messages to
            table,                                              // My understanding of the state
            assignment_time: vec![SpeculativeDepth::Unassigned; vars],   // At what speculative depth was each variable assigned (0=unassigned)
            var_updates: Vec::new(),                            // Which clause are we currently processing
            parallel_clauses,                                   // How many clauses are checked per clock cycle
            speculative_branches: Vec::new(),                   // What is the speculative of newly assigned variables (some optimizaiton to use this as both a speculative and unit propagation buffer)
            state: NodeState::AwaitingFork,                     // make sure to start at false except for the first node so they don't repeat work
            incoming_message: None,                             // 
            watchdog: Watchdog::new(0, 500),
            pipeline_size: 1,
            unit_propagation: Vec::new(),
        }
    }

    /// Adds a neighbour to the node, used by the topology to set up the network
    pub fn add_neighbor(&mut self, id: NodeId) {
        self.neighbors.push(id);
    }

    /// Removes a neighbour from the node, used by the topology to tear down the network (remove certain connections)
    pub fn remove_neighbor(&mut self, id: NodeId) {
        self.neighbors.retain(|&n| n != id);
    }

    /// Activates the node -- sets it to "busy"
    pub fn activate(&mut self) {self.state = NodeState::Busy;}

    // ----- getters ----- //
    /// 
    pub fn busy(&self) -> bool {return self.state != NodeState::AwaitingFork}


    fn get_next_var(&self) -> Option<usize>{
        return self.assignment_time.iter().position(|x| *x == SpeculativeDepth::Unassigned) // For now get the index of the first unassigned variable
    }


    fn get_deepest_speculation(&self) -> VarId {
        let mut max = 0;
        for var in self.assignment_time.iter() {
            match var {
                SpeculativeDepth::Depth(depth, _) => {
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
                assert!(self.speculative_branches.is_empty(), "Node {} received fork while still processing", self.id);
                assert!(self.unit_propagation.is_empty(), "Node {} received fork while still processing unit props", self.id);
                assert!(self.var_updates.is_empty(), "Node {} received fork while still processing var updates", self.id);
                self.watchdog.check(clock);
                assert!(self.speculative_branches.is_empty(), "Node {} received fork while still processing", self.id);
                self.table = table;
                assert!(self.assignment_time.len() == assigned_vars.len(), "nodes have different number of variables");
                self.assignment_time = assigned_vars;
                let var = self.get_next_var().expect("Forked SAT problem!") as VarId;
                self.substitute(var, true, false, self.get_deepest_speculation()+1);
            },
            (NodeState::Busy, None) => {
                if DEBUG_PRINT {
                    println!("Assignment time: {:?}", self.assignment_time);
                }
                if self.var_updates.len() < self.pipeline_size {
                    self.branch(clock, network, busy_nodes);
                }
                let Self {   // Doing bs to avoid borrowing issues
                    table, 
                    var_updates, 
                    assignment_time, 
                    unit_propagation ,
                    ..
                } = self;
                let mut unsat_depth = None;
                var_updates.retain(|var_update| var_update.clause_index < table.num_clauses);
                for var_update in var_updates.iter_mut() {
                    for _ in 0..self.parallel_clauses {
                        let success = Self::process_clause(table, var_update, assignment_time, unit_propagation);
                        if !success {
                            if DEBUG_PRINT {
                                let clause_state = table.clause_table[var_update.clause_index].iter().map(|(t, s)| (t.var, t.negated, s)).collect::<Vec<_>>();
                                println!("Node {} found unsat at depth {} in clause {} with assignments {:?} & clause_state {:?}", self.id, var_update.depth, var_update.clause_index, assignment_time, clause_state);
                            }
                            unsat_depth = Some(var_update.depth);
                            break;
                        }
                        var_update.clause_index += 1;
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
            if let SpeculativeDepth::Depth(prev_depth, prev_assign) = self.assignment_time[var_id as usize] {
                if prev_assign == assignment {
                    // we are already assigned this value
                    return;
                } else if prev_depth > speculative_depth {
                    // we are already assigned this value
                    self.unsat(speculative_depth);
                    return;
                }
            } else {
                self.substitute(var_id, assignment, false, speculative_depth);
            }
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
        } else if self.var_updates.is_empty() {
            if DEBUG_PRINT {
                println!("Node {} is SAT", self.id);
            }
            // we are done done because there is no more work
            // TODO: check that the unsat substitutes fast enough
            self.sat(clock, network);
        }
        else {
            // println!("WHY ARE WE HERE!");
            // println!()
        }
    }

    fn partner_branch(&mut self, clock: u64, network: &mut MessageQueue, var: VarId, neighbor_id: NodeId) {
        assert!(self.state == NodeState::Busy, "Node {} is not in busy state", self.id);
        
        // copy the CNF state and send the fork. Then continue with the other branch 
        let fork_msg = Message::Fork {table: self.table.clone(), assigned_vars: self.assignment_time.clone()};
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
        self.assignment_time[var as usize] = SpeculativeDepth::Depth(speculative_depth, assignment);
        if reset {
            self.assignment_time.iter_mut().for_each(|x| {
                if let SpeculativeDepth::Depth(depth, _) = x {
                    if *depth > speculative_depth {
                        *x = SpeculativeDepth::Unassigned;
                    }
                }
            });
        }
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
        let mut iter = table.clause_table[var_update.clause_index].iter()
            .map(|(Term { var, negated }, _)| {
                if *var == var_update.var_id {
                    if *negated == !var_update.assignment {
                        TermUpdate::True
                    } else {
                        TermUpdate::False
                    }
                } else if let SpeculativeDepth::Unassigned = update_buffer[*var as usize] {
                    // if DEBUG_PRINT {
                    //     println!("Node {} (depth: {}) resetting var {} in clause {}", var_update.var_id, var_depth, var, var_update.clause_index);
                    // }
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
        for ((t, term), result) in current_clause.iter_mut().zip(mask) {
            match result {
                TermUpdate::True => { // true in clause makes the whole clause true
                    *term = TermState::True;
                },
                TermUpdate::False => {
                    *term = TermState::False;
                },
                TermUpdate::Reset => {
                    *term = TermState::Symbolic;
                },
                TermUpdate::Unchanged => {}
            }
        }
        
        // check results
        if current_clause.iter().any(|(_, state)| *state == TermState::True) {
            // clause is satisfied, do nothing
            return true;
        } else if current_clause.iter().all(|(_, state)| *state == TermState::False) {
            // self.unsat(var_update.depth);
            return false;
        } else if current_clause.iter().filter(|(_, state)| *state == TermState::Symbolic).count() == 1 {
            let (term, sym) = current_clause.iter().find(|(_, state)| *state == TermState::Symbolic).unwrap();
            assert!(*sym == TermState::Symbolic, "Found non-symbolic term in unit propagation");
            if DEBUG_PRINT {
                println!("Node {} found unit propagation in clause {} with term {:?}", var_update.var_id, var_update.clause_index, term);
            }
            unit_props.push(UnitPropagation {
                speculative_depth: var_update.depth,
                var_id: term.var,
                assignment: !term.negated,
            });
        }
        return true;
    }

    // ----- termination ----- //
    fn clear_state(&mut self) {
        if DEBUG_PRINT {
            println!("Node {} clearing state", self.id);
        }
        self.state = NodeState::AwaitingFork; 
        self.var_updates.clear();
        // self.update.clear();
        self.unit_propagation.clear();
        self.speculative_branches.clear();
    }
    fn unsat(&mut self, speculative_depth: VarId) {
        self.var_updates.retain(|var_update| var_update.depth < speculative_depth);
        if self.speculative_branches.is_empty() { 
            self.clear_state();
        } else {
            self.backtrack();
        }
    }

    fn backtrack(&mut self) {
        self.unit_propagation.clear();
        let var = self.speculative_branches.pop().expect("No branches to backtrack");
        let (current_depth, assignment) = if let Some(spec) = self.speculative_branches.last() {
            if let SpeculativeDepth::Depth(depth, assignment) = self.assignment_time[*spec as usize] {
                (depth, !assignment)
            } else {
                panic!("Speculating on unassigned variable");
            }
        } else {
            match self.assignment_time[var as usize] {
                SpeculativeDepth::Depth(depth, assignment) => (0, !assignment),
                _ => panic!("Were speculating on unassigned variable"),
            }
        };

        if DEBUG_PRINT {
            println!("Node {} backtracking to var {} at depth {}", self.id, var, current_depth);
        }
        self.substitute(var, assignment, true,  current_depth);
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