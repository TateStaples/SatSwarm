/*
This module defines the `Node` structure and its associated methods.

The `Node` structure represents a node in a network topology with dynamic neighbors.
Each neighbor can receive different types of messages for work distribution and coordination
in the SAT solver.

Message types: 
- Fork: Contains CNF assignment buffer state and variable assignments
- Success: Signal to broadcast SAT solution
*/


use core::panic;
use std::{collections::HashMap, fmt::Debug};

use crate::{get_clock, structures::clause_table::{Term, TermState}, DEBUG_PRINT, GLOBAL_CLOCK};

use super::{clause_table::ClauseTable, message::{Message, MessageDestination, MessageQueue, TermUpdate, Watchdog}};

pub type NodeId = usize;
pub type VarId = u8;
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
    done: bool
}
impl SatSwarm {
    pub fn _blank(clause_table: ClauseTable) -> Self {
        SatSwarm {
            arena: Arena { nodes: Vec::new() },
            clauses: clause_table,
            messages: MessageQueue::new(),
            done: false,
            start_time: *get_clock()
        }
    }

    pub fn grid(clause_table: ClauseTable, rows: usize, cols: usize)  -> Self {
        let mut arena = Arena { nodes: Vec::with_capacity(rows * cols) };
        for i in 0..rows {
            for j in 0..cols {
                let id = arena.nodes.len();
                arena.nodes.push(Node::new(id, clause_table.clone()));
                if i > 0 {
                    arena.add_neighbor(id, id - cols);
                }
                if j > 0 {
                    arena.add_neighbor(id, id - 1);
                }
            }
        }
        SatSwarm {
            arena,
            clauses: clause_table,
            messages: MessageQueue::new(),
            done: false,
            start_time: *get_clock(),
        }
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
        // FIXME: I think this format makes it possible for forking collisions (this applies to the other branch to)

        // Then, apply the updates
        for (node_id, free_neighbors) in updates {
            let node = self.arena.get_node_mut(node_id);
            node.clock_update(free_neighbors, &mut self.messages);
        }
        self.invariants();
    }

    pub fn test_satisfiability(&mut self) -> (bool, i32) {
        let start = *get_clock();
        self.arena.get_node_mut(0).activate();
        while !self.done && self.arena.nodes.iter().any(|node| node.busy()) {
            self.clock_update();
        }
        let end = *get_clock();
        let time = end - start;
        (self.done, time as i32)
    }
    fn send_message(&mut self, from: MessageDestination, to: MessageDestination, message: Message) {
        match to {
            MessageDestination::Neighbor(id) => {
                self.arena.get_node_mut(id).recieve_message(from, message);
            },
            MessageDestination::Broadcast => {
                // the only broadcast rn is success which makes the whole network done
                assert!(self.done == false, "Broadcasting success when already done");
                match (from, message) {
                    (MessageDestination::Neighbor(id), Message::Success) => {
                        // print in sorted order of keys
                        let node: &Node = self.arena.get_node(id);
                        let model = self.recover_model(id);
                        let mut labels: Vec<_> = model.clone().into_iter().collect();
                        labels.sort_by_key(|&(var, _)| var);
                        println!("Model: {:?}", labels);
                        
                        for clause in self.clauses.clause_table.iter() {
                            let mut found_true = false;
                            let mut clause_str = String::from("|\t");
                            for (term, term_state) in clause.iter() {
                                let term_str = match term {
                                    Term {var, negated} => {
                                        let val = match model.get( &var) {
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
            }
        }
    }
    fn invariants(&self) {
        // possible add invariants here to check for correctness
    }
    fn recover_model(&self, id: NodeId) -> HashMap<VarId, bool> {
        let node = self.arena.get_node(id);
        let mut model = HashMap::new();
        model.insert(0, false);  // first variable is always false
        
        for clause in self.arena.get_node(id).table.clause_table.iter() {
            for (term, state) in clause.iter() {
                match *state {
                    TermState::True => {
                        model.insert(term.var, !term.negated);
                    },
                    TermState::False => {
                        model.insert(term.var, term.negated);
                    },
                    _ => {
                        model.insert(term.var, false);
                    }     
                }
            }
        }
        model
        // node.model.clone()
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NodeState {  
    Busy,
    Branching,
    AwaitingFork,
    RecievingFork,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SpeculativeDepth { Depth(VarId), Unassigned}
pub struct Node {
    id: NodeId,
    pipeline_width: usize,
    neighbors: Vec<NodeId>,
    table: ClauseTable,
    update: Vec<SpeculativeDepth>,  // TODO: possible replace with with a vec
    clause_index: usize,  // TODO: this needs to be expanded for pipelining
    state: NodeState, 
    speculative_branches: Vec<VarId>,
    incoming_message: Option<Message>,
    watchdog: Watchdog,
}
// TODO: update SAT to be when all variables are set (this should be a rare case)
impl Node {
    pub fn new(id: NodeId, table: ClauseTable) -> Self {
        let vars = table.num_vars;
        Node {
            id,                                                 // My id
            neighbors: Vec::new(),                              // NodeId of nodes that we can send fork messages to
            table,                                              // My understanding of the state
            update: vec![SpeculativeDepth::Unassigned; vars],   // At what speculative depth was each variable assigned (0=unassigned)
            clause_index: 0,                                    // Which clause are we currently processing
            pipeline_width: 1,                                  // How many clauses are checked per clock cycle
            speculative_branches: Vec::new(),                   // What is the speculative of newly assigned variables (some optimizaiton to use this as both a speculative and unit propagation buffer)
            state: NodeState::AwaitingFork,                     // make sure to start at false except for the first node so they don't repeat work
            incoming_message: None,                             // 
            watchdog: Watchdog::new(5),
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
            (NodeState::Branching, None) => {
                self.watchdog.check();
                if let Some(var) = self.get_next_var() {
                    let var = var as VarId;
                    if let Some(neighbor_id) = free_neighbors.first() {
                        self.partner_branch(network, var, *neighbor_id);
                    } else {
                        self.speculative_branch(var);
                    }
                } else {
                    self.sat(network);
                }
            },
            (NodeState::RecievingFork, Some(Message::Fork {table, assigned_vars})) => {
                self.watchdog.check();
                assert!(self.speculative_branches.is_empty(), "Node {} received fork while still processing", self.id);
                self.table = table;
                assert!(self.update.len() == assigned_vars.len(), "nodes have different number of variables");
                self.update = assigned_vars;
                let var = self.get_next_var().expect("Forked SAT problem!") as VarId;
                self.substitute(var, true, false);
            },
            (NodeState::Busy, None) => {
                for _ in 0..self.pipeline_width {
                    self.process_clause();
                    if self.clause_index >= self.table.num_clauses || self.state != NodeState::Busy {
                        self.state = NodeState::Branching;
                        break;
                    }
                }
                self.watchdog.check();
            },
            (NodeState::AwaitingFork, None) => {
                self.watchdog.check();
            },  // do nothing, keep waiting
            (_, m) => panic!("{:?} received unexpected message {:?}", self, m)
        }
    }

    fn get_next_var(&self) -> Option<usize>{
        return self.update.iter().position(|x| *x == SpeculativeDepth::Unassigned) // For now get the index of the first unassigned variable
    }

    fn partner_branch(&mut self, network: &mut MessageQueue, var: VarId, neighbor_id: NodeId) {
        assert!(self.state == NodeState::Branching, "Node {} is not in branching state", self.id);
        
        // copy the CNF state and send the fork. Then continue with the other branch 
        let fork_msg = Message::Fork {table: self.table.clone(), assigned_vars: self.update.clone()};
        self.send_message(network, MessageDestination::Neighbor(neighbor_id), fork_msg);  

        // now substitute the variable here
        self.substitute(var, false, false);
    }

    fn speculative_branch(&mut self, var: VarId) {
        assert!(self.state == NodeState::Branching, "Node {} is not in branching state", self.id);
        self.speculative_branches.push(var);  //  TODO: I think this can be removedd
        self.substitute(var, false, false);
    }

    fn substitute(&mut self, var: VarId, assignment: bool, reset: bool) {
        todo!();
        self.init_processing();
    }
    fn init_processing(&mut self) {
        assert!(self.state == NodeState::RecievingFork || self.state == NodeState::Branching, "Node {} is not ready to process", self.id);
        self.state = NodeState::Busy;
        self.clause_index = 0;
    }

    fn mask(&self) -> [TermUpdate; CLAUSE_LENGTH] {
        todo!()
    }

    fn process_clause(&mut self) {
        assert!(self.clause_index < self.table.clause_table.len(), "Node {} is reading past the end of the clause", self.id);
        // later optimizations mean we can fast forward through tautologies
        let mask = self.mask();
        let current_clause = &mut self.table.clause_table[self.clause_index];

        // assign the variable
        let mut _symbolic_count = 0; //  potentially useful for later optimizations (unit propagation)
        for ((_, term), result) in current_clause.iter_mut().zip(mask) {
            _symbolic_count += if *term == TermState::Symbolic {1} else {0};
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
        if current_clause.iter().all(|(_, state)| *state == TermState::False) {
            self.unsat();
            return;
        } else if _symbolic_count == 1 {
            // TODO: unit propagation
        }
        

        self.clause_index += 1;
        
    }
    
    fn end_processing(&mut self) {
        assert!(self.state == NodeState::Busy, "Node {} is not processing", self.id);
        assert!(self.clause_index == self.table.clause_table.len() - 1, "Node {} is not at the end of the clause", self.id);
        self.state = NodeState::Branching;
    }

    fn unsat(&mut self) {
        if self.speculative_branches.is_empty() {
            self.state = NodeState::AwaitingFork; 
        } else {
            self.backtrack();
        }
    }

    fn backtrack(&mut self) {
        let var = self.speculative_branches.pop().expect("No branches to backtrack");
        self.state = NodeState::Branching; // this is changed right back to processing but for passing invariant checks
        self.substitute(var, true, true);
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

    fn send_message(&self, network: &mut MessageQueue, dest: MessageDestination, message: Message) {
        network.start_message(MessageDestination::Neighbor(self.id), dest, message);
    }
} impl Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node id: {}, state: {:?}, neighbors: {:?}", self.id, self.state, self.neighbors)
    }
    
}