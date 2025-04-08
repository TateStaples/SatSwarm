use crate::DEBUG_PRINT;

use super::{clause_table::ClauseTable, message::{Message, MessageDestination, MessageQueue}, node::{Node, NodeId}};

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
            start_time: 0
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
            start_time: 0
        }
    }

    fn clock_update(&mut self) {
        if DEBUG_PRINT {println!("Clock TICK:");}
        // print clock every 100,000 cycles
        if self. % 100_000 == 0 {
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
        self.arena.get_node_mut(0).activate();
        while !self.done && self.arena.nodes.iter().any(|node| node.busy()) {
            self.clock_update();
        }
        let time = todo!();
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

