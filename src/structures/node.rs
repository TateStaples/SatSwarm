/*
This module defines the `Node` structure and its associated methods.

The `Node` structure represents a node in a network topology with dynamic neighbors.
Each neighbor can receive different types of messages for work distribution and coordination
in the SAT solver.

Message types: 
- Fork: Contains CNF assignment buffer state and variable assignments
- Success: Signal to broadcast SAT solution
*/

// TODO: figure out how to handle remaining Clause masks after finding final UNSAT in a node -> possibly fixed by adding abort message and node state
// FIXME: tautology check is not working correctly (resets break the check as trues look like false) -> fixed by adding True to TermState

use core::panic;
use std::{collections::HashMap, fmt::Debug};

use crate::{get_clock, TestResult, TestConfig, Topology, DEBUG_PRINT, GLOBAL_CLOCK};

use super::{clause_table::{self, ClauseTable}, message::{Message, MessageDestination, MessageQueue, TermUpdate, Watchdog}};

pub type NodeId = usize;
pub type VarId = u8;
pub type ClockCycle = u64;
pub const CLAUSE_LENGTH: usize = 3;

struct Arena {
    nodes: Vec<Node>,
} impl Arena {
    pub fn new() -> Self {
        Arena {
            nodes: Vec::new()
        }
    }

    pub fn from_nodes(nodes: Vec<Node>) -> Self {
        Arena {
            nodes
        }
    }


    pub fn get_node(&self, id: NodeId) -> &Node {self.nodes.get(id).expect("Node not found")}
    pub fn get_node_mut(&mut self, id: NodeId) -> &mut Node {self.nodes.get_mut(id).expect("Node not found")}
    pub fn get_node_opt(&self, id: NodeId) -> Option<&Node> {self.nodes.get(id)}
    pub fn get_node_mut_opt(&mut self, id: NodeId) -> Option<&mut Node> {self.nodes.get_mut(id)}

    pub fn add_neighbor(&mut self, node_id: NodeId, neighbor_id: NodeId) {
        let n1 = self.nodes.get_mut(node_id).expect("Node not found");
        n1.add_neighbor(neighbor_id);
        
        let n2 = self.nodes.get_mut(neighbor_id).expect("Neighbor not found");
        n2.add_neighbor(node_id);
    }

    pub fn remove_neighbor(&mut self, node_id: NodeId, neighbor_id: NodeId) {
        let n1 = self.nodes.get_mut(node_id).expect("Node not found");
        n1.remove_neighbor(neighbor_id);

        let n2 = self.nodes.get_mut(neighbor_id).expect("Neighbor not found");
        n2.remove_neighbor(node_id);
    }
}
pub struct SatSwarm {
    arena: Arena,
    clauses: ClauseTable,
    messages: MessageQueue,
    start_time: u64,
    done: bool,
    idle_cycles: u64,
    busy_cycles: u64,
}
impl SatSwarm {
    fn build(arena: Arena, clause_table: ClauseTable) -> Self {
        SatSwarm {
            arena,
            clauses: clause_table,
            messages: MessageQueue::new(),
            done: false,
            start_time: *get_clock(),
            idle_cycles: 0,
            busy_cycles: 0,
        }
    }

    pub fn _blank(clause_table: ClauseTable) -> Self {
        SatSwarm::build(Arena { nodes: Vec::new() }, clause_table)
    }
    pub fn generate(clause_table: ClauseTable, config: &TestConfig) -> Self {
        let mut swarm = match config.topology {
            Topology::Grid(rows, cols) => SatSwarm::grid(clause_table, rows, cols),
            Topology::Torus(rows, cols) => SatSwarm::torus(clause_table, rows, cols),
            Topology::Dense(num_nodes) => SatSwarm::dense(clause_table, num_nodes),
        };
        swarm.clauses.set_bandwidth(config.table_bandwidth);
        swarm.messages.set_bandwidth(config.node_bandwidth);
        swarm
    }
    pub fn grid(clause_table: ClauseTable, rows: usize, cols: usize)  -> Self {
        let mut arena = Arena { nodes: Vec::with_capacity(rows * cols) };
        let blank_state = clause_table.get_blank_state();
        for i in 0..rows {
            for j in 0..cols {
                let id = arena.nodes.len();
                arena.nodes.push(Node::new(id, blank_state.clone()));
                if i > 0 {
                    arena.add_neighbor(id, id - cols);
                }
                if j > 0 {
                    arena.add_neighbor(id, id - 1);
                }
            }
        }
        SatSwarm::build(arena, clause_table)
    }

    pub fn torus(clause_table: ClauseTable, rows: usize, cols: usize)  -> Self {
        let mut arena = Arena { nodes: Vec::with_capacity(rows * cols) };
        let blank_state = clause_table.get_blank_state();
        for i in 0..rows {
            for j in 0..cols {
                let id = arena.nodes.len();
                assert!(id == i * cols + j, "Node id {} does not match expected id {}", id, i * cols + j);
                arena.nodes.push(Node::new(id, blank_state.clone()));
                // Connect to the node above (wrap around for torus)
                if i > 0 {
                    arena.add_neighbor(id, id - cols);
                } else {
                    arena.add_neighbor(id, id + (rows - 1) * cols);
                }
                // Connect to the node to the left (wrap around for torus)
                if j > 0 {
                    arena.add_neighbor(id, id - 1);
                } else {
                    arena.add_neighbor(id, id + cols - 1);
                }
            }
        }
        SatSwarm::build(arena, clause_table)
    }

    pub fn dense(clause_table: ClauseTable, num_nodes: usize) -> Self {
        let mut arena = Arena { nodes: Vec::with_capacity(num_nodes) };
        let blank_state = clause_table.get_blank_state();
        for id in 0..num_nodes {
            arena.nodes.push(Node::new(id, blank_state.clone()));
        }
        for i in 0..num_nodes {
            for j in (i + 1)..num_nodes {
                arena.add_neighbor(i, j);
            }
        }
        SatSwarm::build(arena, clause_table)
    }

    fn clock_tick() {
        // separate function to make sure the clock is updated correctly (unsafe in multithreaded environments)
        unsafe {
            GLOBAL_CLOCK += 1;
        }
    }
    fn clock_update(&mut self) {
        SatSwarm::clock_tick();
        if DEBUG_PRINT {println!("Clock TICK:");}
        // print clock every 100,000 cycles
        if *get_clock() % 100_000 == 0 {
            // print clock and late_update of all nodes
            // for node in self.arena.nodes.iter() {
            //     print!("Node {} @ {}, ", node.id, node.last_update );
            // }
            if *get_clock() - self.start_time >= 50_000_000 {
                self.done = true;
                println!("Timeout after 50_000_000 cycles");
            }
            println!("Clock: {}", *get_clock());
        }
        for (from, to, msg) in self.messages.pop_message() {
            if DEBUG_PRINT {println!("Message: {:?} from {:?} to {:?}", msg, from, to);}
            self.send_message(from, to, msg);
        }
        self.clauses.clock_update(&mut self.messages);

        // First, collect the data we need
        let updates: Vec<(NodeId, Vec<NodeId>)> = self.arena.nodes.iter()
            .map(|node| {
                let free_neighbor_ids: Vec<NodeId> = node.neighbors.iter()
                    .filter(|&&n| !self.arena.get_node(n).busy())
                    .copied()
                    .collect();
                (node.id, free_neighbor_ids)
            })
            .collect();

        // Then, apply the updates
        for (node_id, free_neighbors) in updates {
            let free_neighbors: Vec<NodeId> = free_neighbors.iter()
                .filter(|&&n| !self.arena.get_node(n).busy())
                .copied()
                .collect();
            let node = self.arena.get_node_mut(node_id);
            if node.busy() {
                self.busy_cycles += 1;
            } else {
                self.idle_cycles += 1;
            }
            node.clock_update(free_neighbors, &mut self.messages);
        }
        self.invariants();
    }

    pub fn test_satisfiability(&mut self) -> TestResult {
        let start = *get_clock();
        self.arena.get_node_mut(0).activate();
        while !self.done && self.arena.nodes.iter().any(|node| node.busy()) {
            self.clock_update();
        }
        let end = *get_clock();
        let time = end - start;
        if true {
            println!("Done: {}", self.done);
            println!("Busy cycles: {}", self.busy_cycles);
            println!("Idle cycles: {}", self.idle_cycles);
        }
        TestResult {
            simulated_result: self.done,
            simulated_cycles: time,
            cycles_busy: self.busy_cycles,
            cycles_idle: self.idle_cycles,
        }
    }
    fn send_message(&mut self, from: MessageDestination, to: MessageDestination, message: Message) {
        match to {
            MessageDestination::Neighbor(id) => {
                self.arena.get_node_mut(id).recieve_message(from, message);
            },
            MessageDestination::Broadcast => {
                // the only broadcast rn is success which makes the whole network done
                assert!(self.done == false, "Broadcasting success when already done");
                assert!(message == Message::Success, "Unexpected broadcast message");
                match from {
                    MessageDestination::Neighbor(id) => {
                        // print in sorted order of keys
                        let node: &Node = self.arena.get_node(id);
                        let model = self.recover_model(id);
                        let mut labels: Vec<_> = model.clone().into_iter().collect();
                        labels.sort_by_key(|&(var, _)| var);
                        println!("Model: {:?}", labels);
                        
                        for (clause, clause_state) in self.clauses.clause_table.iter().zip(node.table.iter()) {
                            let mut found_true = false;
                            let mut clause_str = String::from("|\t");
                            for (term, term_state) in clause.iter().zip(clause_state.iter()) {
                                let term_str = match term {
                                    clause_table::Term {var, negated} => {
                                        let val = match model.get(var) {
                                            Some(val) => val,
                                            None => panic!("Variable {} not found in model", var)
                                        };
                                        let term_val = if *negated { !val } else { *val };
                                        if term_val {
                                            found_true = true;
                                        }
                                        match term_state {
                                            TermState::True => {
                                                assert!(term_val, "Term {:?} is not consistent with term state {:?}", term, term_state);
                                            },
                                            TermState::False => {
                                                assert!(!term_val, "Term {:?} is not consistent with term state {:?}", term, term_state);
                                            },
                                            TermState::Symbolic => {
                                            }
                                            
                                        }
                                        // assert!(if term_val { *term_state == TermState::True } else { *term_state == TermState::False }, "{:?} is not consistent with term state {:?}", term, term_state);

                                        if *negated {
                                            format!("!{} ({})", var, !val)
                                        } else {
                                            format!("{} ({})", var, val)
                                        }
                                    }
                                };
                                clause_str.push_str(&term_str);
                                clause_str.push_str("\t|\t");
                            }
                            println!("Clause: {}", clause_str);
                            assert!(found_true, "Clause is not satisfied");
                        }

                    },
                    _ => panic!("Broadcast message from unexpected source")
                };
                self.done = true;
            },
            MessageDestination::ClauseTable => {
                self.clauses.recieve_message(from, message);
            }
        }
    }
    fn invariants(&self) {
        // possible add invariants here to check for correctness
    }
    fn recover_model(&self, id: NodeId) -> HashMap<VarId, bool> {
        let node = self.arena.get_node(id);
        let table = self.clauses.clone_table();
        let mut model = HashMap::new();
        model.insert(0, false);  // first variable is always false
        for (i, state) in node.table.iter().enumerate() {
            for (j, term) in state.iter().enumerate() {
                let clause = table[i][j];
                match term {
                    TermState::True => {
                        model.insert(clause.var, !clause.negated);
                    },
                    TermState::False => {
                        model.insert(clause.var, clause.negated);
                    },
                    _ => {
                        model.insert(clause.var, false);
                    }     
                }
            }
        }
        model
        // node.model.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermState {False, True, Symbolic} // True is not needed since the clause is satisfied when any term is true
pub type ClauseState = [TermState; CLAUSE_LENGTH];
pub type CNFState = Vec<ClauseState>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NodeState {  
    ProcessingClauses,
    Branching,
    AwaitingFork,
    RecievingFork,
    AbortingProcess,
}
pub struct Node {
    id: NodeId,
    neighbors: Vec<NodeId>,
    table: CNFState,
    last_update: VarId,
    clause_index: usize,
    sat_flag: bool,
    state: NodeState, 
    speculative_branches: Vec<VarId>,
    incoming_message: Option<Message>,
    watchdog: Watchdog,
}
impl Node {
    pub fn new(id: NodeId, table: CNFState) -> Self {
        Node {
            id,
            neighbors: Vec::new(),
            table,
            last_update: 0,
            clause_index: 0,
            sat_flag: true,
            speculative_branches: Vec::new(),
            state: NodeState::AwaitingFork,  // make sure to start at false except for the first node so they don't repeat work
            incoming_message: None,
            watchdog: Watchdog::new(150),
        }
    }

    pub fn add_neighbor(&mut self, id: NodeId) {
        self.neighbors.push(id);
    }

    pub fn remove_neighbor(&mut self, id: NodeId) {
        self.neighbors.retain(|&n| n != id);
    }

    pub fn busy(&self) -> bool {return self.state != NodeState::AwaitingFork}
    pub fn activate(&mut self) {self.state = NodeState::Branching}

    pub fn clock_update(&mut self, free_neighbors: Vec<NodeId>, network: &mut MessageQueue) {
        let msg = std::mem::replace(&mut self.incoming_message, None);
        match (&self.state, msg) {
            (NodeState::Branching, null_msg) => {
                self.watchdog.check();
                assert!(null_msg.is_none(), "Node {} received unexpected message", self.id);
                if let Some(neighbor_id) = free_neighbors.first() {
                    self.partner_branch(network, *neighbor_id);
                } else {
                    self.speculative_branch(network);
                }
            },
            (NodeState::RecievingFork, Some(Message::Fork {cnf_state, assigned_vars})) => {
                self.watchdog.check();
                assert!(self.speculative_branches.is_empty(), "Node {} received fork while still processing", self.id);
                self.table = cnf_state;
                self.last_update = assigned_vars;
                self.substitute(network, true, false);
            },
            (NodeState::ProcessingClauses, Some(Message::SubstitutionMask {mask})) => {
                self.watchdog.check();
                self.process_clause(network, mask);
            },
            (NodeState::ProcessingClauses, Some(Message::VariableNotFound)) => {
                self.watchdog.check();
                assert!(self.clause_index == 0, "Node {} received VariableNotFound but not at the first clause", self.id);
                self.unsat(network);
            },
            (NodeState::ProcessingClauses, None) => {  // Waiting the get the mask for the first clause
                self.watchdog.peek();
                assert!(self.clause_index == 0, "Node {} is waiting for mask but not at the first clause", self.id);
            },
            (NodeState::AbortingProcess, None) => { // This model assumes no possible gaps in network. Realistically, we should have a timeout or NACK
                self.watchdog.check();
                self.unsat(network);
            },
            (NodeState::AbortingProcess, Some(Message::SubstitutionMask{..})) => {  // round-trip cancellation hasn't propagated yet
                self.watchdog.check();
            },
            (NodeState::AwaitingFork, _) => {
                self.watchdog.check();
            },  // do nothing
            (_, m) => panic!("{:?} received unexpected message {:?}", self, m)
        }
    }

    fn partner_branch(&mut self, network: &mut MessageQueue, neighbor_id: NodeId) {
        assert!(self.state == NodeState::Branching, "Node {} is not in branching state", self.id);
        // copy the CNF state and send the fork. Then continue with the other branch 
        let new_state = self.table.clone();
        let fork_msg = Message::Fork {cnf_state: new_state, assigned_vars: self.last_update};
        self.send_message(network, MessageDestination::Neighbor(neighbor_id), fork_msg);  

        // now substitute the variable here
        self.substitute(network, false, false);
    }

    fn speculative_branch(&mut self, network: &mut MessageQueue) {
        assert!(self.state == NodeState::Branching, "Node {} is not in branching state", self.id);
        self.speculative_branches.push(self.last_update);
        self.substitute(network, false, false);
    }

    fn substitute(&mut self, network: &mut MessageQueue, assignment: bool, reset: bool) {
        let msg = Message::SubsitutionQuery {id: self.last_update, assignment, reset};
        self.send_message(network, MessageDestination::ClauseTable, msg);
        self.last_update += 1;
        self.init_processing();
    }
    fn init_processing(&mut self) {
        assert!(self.state == NodeState::RecievingFork || self.state == NodeState::Branching, "Node {} is not ready to process", self.id);
        self.state = NodeState::ProcessingClauses;
        self.clause_index = 0;
        self.sat_flag = true;
    }

    fn process_clause(&mut self, parent: &mut MessageQueue, mask: [TermUpdate; CLAUSE_LENGTH]) {
        // check if the clause is a tautology
        let current_clause = &self.table[self.clause_index];
        if !self.check_tautology(current_clause, &mask) || true {  // later optimizations mean we can fast forward through tautologies
            let mut current_clause = &mut self.table[self.clause_index];

            // assign the variable
            let mut unsat = true;
            let mut _symbolic_count = 0; //  potentially useful for later optimizations (unit propagation)
            for (term, result) in current_clause.iter_mut().zip(mask.iter()) {
                _symbolic_count += if *term == TermState::Symbolic {1} else {0};
                match result {
                    TermUpdate::True => { // true in clause makes the whole clause true
                        *term = TermState::True;
                        unsat = false;
                    },
                    TermUpdate::False => {
                        *term = TermState::False;
                    },
                    TermUpdate::Reset => {
                        *term = TermState::Symbolic;
                        unsat = false;
                    },
                    TermUpdate::Unchanged => {
                        if term == &TermState::Symbolic || term == &TermState::True {
                            unsat = false;
                        }
                    }
                }
            }
            if !current_clause.iter().any(|term| *term == TermState::True) {
                self.sat_flag = false;
            } else if unsat {
                assert!(current_clause.iter().all(|term| *term == TermState::False), "Node {} has unsat clause that is not all false", self.id);
                self.abort(parent);
                return;
            }
        }

        if self.clause_index == self.table.len() - 1 {
            self.end_processing(parent);
        } else {
            self.clause_index += 1;
        }
        assert!(self.clause_index < self.table.len(), "Node {} is reading past the end of the clause", self.id);
    }

    fn check_tautology(&self, current_clause: &ClauseState, mask: &[TermUpdate; CLAUSE_LENGTH]) -> bool {
        return current_clause.iter().zip(mask).any(|(term, result)| *term == TermState::True && *result != TermUpdate::Reset);  // later optimization: investigate removing True from term state
    }
    
    fn end_processing(&mut self, parent: &mut MessageQueue) {
        assert!(self.state == NodeState::ProcessingClauses, "Node {} is not processing", self.id);
        assert!(self.clause_index == self.table.len() - 1, "Node {} is not at the end of the clause", self.id);
        if self.sat_flag {
            self.sat(parent);
        } else {
            self.state = NodeState::Branching;
        } 
    }

    fn unsat(&mut self, network: &mut MessageQueue) {
        if self.speculative_branches.is_empty() {
            self.state = NodeState::AwaitingFork; 
        } else {
            self.backtrack(network);
        }
    }
    
    fn abort(&mut self, network: &mut MessageQueue) {
        self.state = NodeState::AbortingProcess;
        self.send_message(network, MessageDestination::ClauseTable, Message::SubstitutionAbort);
    }


    fn backtrack(&mut self, network: &mut MessageQueue) {
        assert!(!self.speculative_branches.is_empty(), "Node {} is backtracking with no branches", self.id);
        self.last_update = self.speculative_branches.pop().expect("No branches to backtrack");
        self.state = NodeState::Branching; // this is changed right back to processing but for passing invariant checks
        self.substitute(network, true, true);
    }

    fn sat(&mut self, network: &mut MessageQueue) {
        println!("Node {} is SAT", self.id);
        self.state = NodeState::AwaitingFork;
        self.send_message(network, MessageDestination::Broadcast, Message::Success);
    }

    pub fn recieve_message(&mut self, from: MessageDestination, message: Message) {
        match from {
            MessageDestination::Neighbor(id) => {
                assert!(self.neighbors.contains(&id), "Node {:?} received message from non-neighbor", self);
            },
            MessageDestination::ClauseTable => {
                assert!(self.state == NodeState::ProcessingClauses || self.state == NodeState::AbortingProcess, "Node {:?} received message from unexpected source", self);  // FIXME: we have a weird edge case where Claues are coming in from previous UNSAT and node starts processing a different fork but misinterprets the message
            },
            _ => panic!("{:?} received unexpected message source", self)
        }
        if self.incoming_message.is_some() {
            // print!("Node {} received message {:?} from {:?} but already has message {:?}", self.id, message, from, msg);
            println!("Node received multiple messages in one cycle");
            panic!("Node {} received multiple messages in one cycle", self.id);
        } else {
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
    }

    fn send_message(&self, network: &mut MessageQueue, dest: MessageDestination, message: Message) {
        network.start_message(MessageDestination::Neighbor(self.id), dest, message);
    }
} impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node id: {}, state: {:?}, neighbors: {:?}", self.id, self.state, self.neighbors)
    }
    
}