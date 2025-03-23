/*
This module defines the `Node` structure and its associated methods.

The `Node` structure represents a node in a network topology with dynamic neighbors.
Each neighbor can receive different types of messages for work distribution and coordination
in the SAT solver.

Message types:
- Fork: Contains CNF assignment buffer state and variable assignments
- Success: Signal to broadcast SAT solution
*/
use std::collections::HashMap;

use super::clause_table::{self, ClauseTable};

pub type NodeId = usize;
pub type VarId = u8;
pub type ClockCycle = u64;
pub const CLAUSE_LENGTH: usize = 3;

pub enum MessageDestination {
    Neighbor(NodeId),
    Broadcast, 
    ClauseTable
} 

struct CircularBuffer<T, const N: usize> {
    buffer: [Vec<T>; N],
    head: usize,
} impl<T, const N: usize> CircularBuffer<T, N> {
    pub fn new() -> Self {
        CircularBuffer {
            buffer: std::array::from_fn::<Vec<T>, N, _>(|_| Vec::new()),
            head: 0
        }
    }

    pub fn push(&mut self, delay: usize, item: T) {
        assert!(delay < N, "Delay too large");
        assert!(delay > 0, "Delay too small");
        self.buffer[self.head].push(item);
    }

    pub fn step(&mut self) {
        self.head = (self.head + 1) % N;
    }

    pub fn pop(&mut self) -> Vec<T> {
        let mut result = Vec::new();
        std::mem::swap(&mut result, &mut self.buffer[self.head]);
        result
    }
}
pub struct MessageQueue {
    clock: ClockCycle,
    queue: CircularBuffer<(MessageDestination, MessageDestination, Message), 64>
}
impl MessageQueue {
    pub fn new() -> Self {
        MessageQueue {
            clock: 0,
            queue: CircularBuffer::new()
        }
    }

    fn set_clock(&mut self, clock: ClockCycle) {
        for _ in self.clock..clock {
            self.queue.step();
        }
        self.clock = clock;
    }

    pub fn start_message(&mut self, from: MessageDestination, to: MessageDestination, message: Message) {
        self.queue.push(1, (from, to, message));  // TODO: add more realis
    }

    pub fn pop_message(&mut self) -> Vec<(MessageDestination, MessageDestination, Message)> {
        self.queue.pop()
    }
}

struct SatSwarm {
    nodes: Vec<Node>,
    clauses: ClauseTable,
    clock: ClockCycle,
    messages: MessageQueue,
    done: bool
}
impl SatSwarm {
    pub fn new(clause_table: ClauseTable) -> Self {
        SatSwarm {
            nodes: Vec::new(),
            clauses: clause_table,
            clock: 0,
            messages: MessageQueue::new(),
            done: false
        }
    }

    fn invariants(&self) {
        assert!(self.messages.clock == self.clock);
    }
    pub fn grid(&mut self, rows: usize, cols: usize) {
        let mut id = 0;
        for r in 0..rows {
            for c in 0..cols {
                let node_id = self.add_node();
                if r > 0 {
                    self.add_neighbor(node_id, id - cols);
                }
                if c > 0 {
                    self.add_neighbor(node_id, id - 1);
                }
                id += 1;
            }
        }
    }

    pub fn clock_update(&mut self) {
        self.clock += 1;
        self.messages.set_clock(self.clock);

        for (from, to, msg) in self.messages.pop_message() {
            match to {
                MessageDestination::Neighbor(id) => {
                    self.nodes.get_mut(id).expect("Node not found")
                        .recieve_message(from, msg);
                },
                MessageDestination::Broadcast => {
                    panic!("Broadcast not implemented");
                },
                MessageDestination::LUT => {
                    self.clauses.recieve_message(from, msg);
                }
            }
        }

        for node in self.nodes.iter_mut() {
            node.clock_update(&mut self.messages);
        }
        self.invariants();
    }

    pub fn add_node(&mut self) -> NodeId {
        let id = self.nodes.len();
        let blank_state = self.clauses.get_blank_state();
        self.nodes.push(Node::new(id, blank_state));
        id
    }

    pub fn get_node(&self, id: NodeId) -> Option<&Node> {self.nodes.get(id)}
    pub fn get_node_mut(&mut self, id: NodeId) -> Option<&mut Node> {self.nodes.get_mut(id)}

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

    pub fn send_message(&mut self, from: MessageDestination, to: MessageDestination, message: Message) {
        todo!()
    }

    pub fn broadcast_success(&mut self, net: &mut SatSwarm, node_id: NodeId) {
        net.done = true;
    }
}

// #[derive(Debug, Clone)]
pub type TermResult = u8; // TODO: 2 bits to represent the result (00=not, 01=neg, 10=pos, 11=X)
pub enum Message {
    Fork {
        cnf_state: ClauseTable,  // CNF assignment buffer state
        assigned_vars: VarId,   // List of already assigned variables (later work can make this more complex)
    },
    Success,
    SubstitutionMask {
        mask: [TermResult; CLAUSE_LENGTH],
    },
    SubsitutionQuery {
        id: VarId
    }
}
pub type ClauseState = [bool; CLAUSE_LENGTH];
pub type CNFState = Vec<ClauseState>;

pub struct Node {
    id: NodeId,
    pub neighbors: Vec<NodeId>,
    pub table: CNFState,
    pub last_update: VarId,
    pub busy = 
    pub incoming_message: Option<Message>
}

impl Node {
    pub fn new(id: NodeId, table: CNFState) -> Self {
        Node {
            id,
            neighbors: Vec::new(),
            table,
            incoming_message: None
        }
    }

    pub fn add_neighbor(&mut self, id: NodeId) {
        self.neighbors.push(id);
    }

    pub fn remove_neighbor(&mut self, id: NodeId) {
        self.neighbors.retain(|&n| n != id);
    }

    pub fn clock_update(&mut self, parent: &mut MessageQueue) {
        // select var to assign
        // check if fork else speculative
    }

    pub fn has_available_neighbor(&self, parent: &SatSwarm) -> bool {
        return self.neighbors.iter().any(|&n| parent.get_node(n).is_some());
    }

    pub fn recieve_message(&mut self, from: MessageDestination, message: Message) {
        if self.incoming_message.is_some() {
            // print!("Node {} received message {:?} from {:?} but already has message {:?}", self.id, message, from, msg);
            println!("Node received multiple messages in one cycle");
        } else {
            self.incoming_message = Some(message);
        }
    }

    pub fn send_message(&self, network: &mut SatSwarm, neighbor_id: NodeId, message: Message) {
        network.send_message(MessageDestination::Neighbor(self.id), MessageDestination::Neighbor(neighbor_id), message);
    }
}