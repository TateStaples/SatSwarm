use std::collections::HashMap;

use crate::{structures::clause_table::{Term, TermState}, TestConfig, TestResult, Topology};

use super::{clause_table::ClauseTable, message::{Message, MessageDestination, MessageQueue}, node::Node, util_types::{NodeId, VarId, DEBUG_PRINT}};


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
            start_time: 0,
            idle_cycles: 0,
            busy_cycles: 0,
        }
    }

    pub fn _blank(clause_table: ClauseTable) -> Self {
        SatSwarm::build(Arena { nodes: Vec::new() }, clause_table)
    }
    pub fn generate(clause_table: ClauseTable, config: &TestConfig) -> Self {
        let mut swarm = match config.topology {
            Topology::Grid(rows, cols) => SatSwarm::grid(clause_table, rows, cols, config.node_bandwidth),
            Topology::Torus(rows, cols) => SatSwarm::torus(clause_table, rows, cols, config.node_bandwidth),
            Topology::Dense(num_nodes) => SatSwarm::dense(clause_table, num_nodes, config.node_bandwidth),
        };
        // swarm.messages.set_bandwidth(config.node_bandwidth);
        swarm
    }
    pub fn grid(clause_table: ClauseTable, rows: usize, cols: usize, node_bandwidth: usize)  -> Self {
        let mut arena = Arena { nodes: Vec::with_capacity(rows * cols) };
        for i in 0..rows {
            for j in 0..cols {
                let id = arena.nodes.len();
                arena.nodes.push(Node::new(id, clause_table.clone(), node_bandwidth));
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

    pub fn torus(clause_table: ClauseTable, rows: usize, cols: usize, node_bandwidth: usize)  -> Self {
        let mut arena = Arena { nodes: Vec::with_capacity(rows * cols) };
        for row_index in 0..rows {
            for col_index in 0..cols {
                let id = arena.nodes.len();
                assert!(id == row_index * cols + col_index, "Node id {} does not match expected id {}", id, row_index * cols + col_index);
                arena.nodes.push(Node::new(id, clause_table.clone(), node_bandwidth));
                // Connect to the node above (wrap around for torus)
                if row_index > 0 {
                    let above = id - cols;
                    arena.add_neighbor(id, above);
                } 
                // Connect to the node to the left (wrap around for torus)
                if col_index > 0 {
                    let left = id - 1;
                    arena.add_neighbor(id, left);
                } 

                if row_index == rows - 1 {
                    let below = col_index;
                    arena.add_neighbor(id, below);
                }
                if col_index == cols - 1 {
                    let right = row_index * cols;
                    arena.add_neighbor(id, right);
                }
            }
        }
        SatSwarm::build(arena, clause_table)
    }

    pub fn dense(clause_table: ClauseTable, num_nodes: usize, node_bandwidth: usize) -> Self {
        let mut arena = Arena { nodes: Vec::with_capacity(num_nodes) };
        for id in 0..num_nodes {
            arena.nodes.push(Node::new(id, clause_table.clone(), node_bandwidth));
        }
        for i in 0..num_nodes {
            for j in (i + 1)..num_nodes {
                arena.add_neighbor(i, j);
            }
        }
        SatSwarm::build(arena, clause_table)
    }

    fn clock_update(&mut self, clock: u64) {
        if DEBUG_PRINT {println!("Clock TICK: {}", clock);}
        // print clock every 100,000 cycles
        if clock % 100_000 == 0 {
            // print clock and late_update of all nodes
            // for node in self.arena.nodes.iter() {
            //     print!("Node {} @ {}, ", node.id, node.last_update );
            // }
            if clock - self.start_time >= 150_000_000 {
                self.done = true;
                println!("Timeout after 150_000_000 cycles");
            }
            println!("Clock: {}", clock);
        }
        for (from, to, msg) in self.messages.pop_message(clock) {
            if DEBUG_PRINT {println!("Message: {:?} from {:?} to {:?}", msg, from, to);}
            self.distribute_message(from, to, msg);
        }

        let mut busy_nodes: Vec<bool> = self.arena.nodes.iter()
            .map(|node| node.busy())
            .collect();
        // Then, apply the updates
        for node in self.arena.nodes.iter_mut() {
            // let node = self.arena.get_node_mut(node_id);
            // assert!(busy_nodes[node.id] == node.busy(), "Node in {} but expected {}", node.busy(), busy_nodes[node.id]);
            if busy_nodes[node.id] {
                self.busy_cycles += 1;
            } else {
                self.idle_cycles += 1;
            }
            node.clock_update(clock, &mut self.messages, &mut busy_nodes);
        }
        self.invariants();
    }

    pub fn test_satisfiability(&mut self) -> TestResult {
        let mut clock = 0;
        self.arena.get_node_mut(0).activate();
        while !self.done && self.arena.nodes.iter().any(|node| node.busy()) {
            self.clock_update(clock);
            clock += 1;
        }
        let time = clock;
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
    fn distribute_message(&mut self, from: MessageDestination, to: MessageDestination, message: Message) {
        match to {
            MessageDestination::Neighbor(id) => {
                self.arena.get_node_mut(id).recieve_message(from, message);
            },
            MessageDestination::Broadcast => {
                // the only broadcast rn is success which makes the whole network done
                // assert!(self.done == false, "Broadcasting success when already done");
                if self.done { return; }
                match (message, from) {
                    (Message::Success, MessageDestination::Neighbor(id)) => {
                        self.done = true;
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
            }
        }
    }
    fn invariants(&self) {
        // possible add invariants here to check for correctness
    }
    fn recover_model(&self, id: NodeId) -> HashMap<VarId, bool> {
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
