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

use super::clause_table::ClauseTable;

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
struct SatSwarm {
    arena: Arena,
    clauses: ClauseTable,
    clock: ClockCycle,
    messages: MessageQueue,
    done: bool
}
impl SatSwarm {
    pub fn new(clause_table: ClauseTable) -> Self {
        SatSwarm {
            arena: Arena { nodes: Vec::new() },
            clauses: clause_table,
            clock: 0,
            messages: MessageQueue::new(),
            done: false
        }
    }

    fn invariants(&self) {
        assert!(self.messages.clock == self.clock);
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
        SatSwarm {
            arena,
            clauses: clause_table,
            clock: 0,
            messages: MessageQueue::new(),
            done: false
        }
    }

    pub fn clock_update(&mut self) {
        self.clock += 1;
        self.messages.set_clock(self.clock);

        for (from, to, msg) in self.messages.pop_message() {
            match to {
                MessageDestination::Neighbor(id) => {
                    self.arena.get_node_mut(id).recieve_message(from, msg);
                },
                MessageDestination::Broadcast => {
                    panic!("Broadcast not implemented");
                },
                MessageDestination::ClauseTable => {
                    self.clauses.recieve_message(from, msg);
                }
            }
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

        // Then, apply the updates
        for (node_id, free_neighbors) in updates {
            let node = self.arena.get_node_mut(node_id);
            node.clock_update(free_neighbors, &mut self.messages);
        }
        self.invariants();
    }

    fn add_neighbor(&mut self, node_id: NodeId, neighbor_id: NodeId) {
        let n1 = self.arena.get_node_mut(node_id);
        n1.add_neighbor(neighbor_id);
        
        let n2 = self.arena.get_node_mut(neighbor_id);
        n2.add_neighbor(node_id);
    }

    fn remove_neighbor(&mut self, node_id: NodeId, neighbor_id: NodeId) {
        let n1 = self.arena.get_node_mut(node_id);
        n1.remove_neighbor(neighbor_id);

        let n2 = self.arena.get_node_mut(neighbor_id);
        n2.remove_neighbor(node_id);
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
                    MessageDestination::Neighbor(id) => id,
                    _ => panic!("Broadcast message from unexpected source")
                };
                self.done = true;
            },
            MessageDestination::ClauseTable => {
                self.clauses.recieve_message(from, message);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TermUpdate {
    Unchanged,
    True,
    False,
    Reset
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Message {
    Fork {
        cnf_state: CNFState,  // CNF assignment buffer state
        assigned_vars: VarId,   // List of already assigned variables (later work can make this more complex)
    },
    Success,
    SubstitutionMask {
        mask: [TermUpdate; CLAUSE_LENGTH],
    },
    SubsitutionQuery {
        id: VarId,
        assignment: bool,  // This seems useful so that when subsituting we can just check if the variable is True or False
        reset: bool,  // whether to flag all subsequently assigned variables as unassigned
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TermState {False, Symbolic} // True is not needed since the clause is satisfied when any term is true
pub type ClauseState = [TermState; CLAUSE_LENGTH];
pub type CNFState = Vec<ClauseState>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NodeState {  
    ProcessingClauses,
    Branching,
    AwaitingFork,
    RecievingFork,
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
    incoming_message: Option<Message>
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
            incoming_message: None
        }
    }

    pub fn add_neighbor(&mut self, id: NodeId) {
        self.neighbors.push(id);
    }

    pub fn remove_neighbor(&mut self, id: NodeId) {
        self.neighbors.retain(|&n| n != id);
    }

    pub fn busy(&self) -> bool {return self.state != NodeState::AwaitingFork}

    pub fn clock_update(&mut self, free_neighbors: Vec<NodeId>, network: &mut MessageQueue) {
        // select var to assign
        // memswap the incoming message out
        let msg = std::mem::replace(&mut self.incoming_message, None);
        match (&self.state, msg) {
            (NodeState::Branching, null_msg) => {
                assert!(null_msg.is_none(), "Node {} received unexpected message", self.id);
                if let Some(neighbor_id) = free_neighbors.first() {
                    self.partner_branch(network, *neighbor_id);
                } else {
                    self.speculative_branch();
                }
            },
            (NodeState::RecievingFork, Some(Message::Fork {cnf_state, assigned_vars})) => {
                self.init_processing();
                self.table = cnf_state;
                self.last_update = assigned_vars;
                self.state = NodeState::Branching; // Takes one clock cycle to process the fork
                let msg = Message::SubsitutionQuery {id: self.last_update, assignment: true, reset: false};  // The forked person always assigns the variable to true
                self.send_message(network, MessageDestination::ClauseTable, msg);
            },
            (NodeState::ProcessingClauses, Some(Message::SubstitutionMask {mask})) => {
                self.process_clause(network, mask);
            },
            (NodeState::AwaitingFork, _) => {}  // do nothing
            _ => panic!("Node {} received unexpected message", self.id)
        }
    }

    fn partner_branch(&mut self, parent: &mut MessageQueue, neighbor_id: NodeId) {
        assert!(self.state == NodeState::Branching, "Node {} is not in branching state", self.id);
        // copy the CNF state and send the fork. Then continue with the other branch 
        let new_state = self.table.clone();
        let fork_msg = Message::Fork {cnf_state: new_state, assigned_vars: self.last_update};
        self.send_message(parent, MessageDestination::Neighbor(neighbor_id), fork_msg);  

        // now substitute the variable here
        let sub_msg = Message::SubsitutionQuery {id: self.last_update, assignment: false, reset: false};  // the forker always assigns the variable to false
        self.send_message(parent, MessageDestination::ClauseTable, sub_msg);
        self.init_processing();
    }

    fn speculative_branch(&mut self) {
        assert!(self.state == NodeState::Branching, "Node {} is not in branching state", self.id);
        self.speculative_branches.push(self.last_update);

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
        if !self.check_tautology(current_clause, &mask) {  // later optimizations mean we can fast forward through tautologies
            // TODO: make sure sat_flag is set correctly
            let mut current_clause = &mut self.table[self.clause_index];

            // assign the variable
            let mut unsat = true;
            let mut symbolic_count = 0; //  potentially useful for later optimizations (unit propagation)
            for (term, result) in current_clause.iter_mut().zip(mask.iter()) {
                symbolic_count += if *term == TermState::Symbolic {1} else {0};
                match result {
                    TermUpdate::True => { // true in clause makes the whole clause true
                        *term = TermState::False;
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
                        unsat = false;
                    }
                }
            }

            // check for UNSAT
            if unsat {
                self.unsat(parent);
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
        return current_clause.iter().zip(mask).all(|(term, result)| *term == TermState::False || *result != TermUpdate::Reset);
    }
    
    fn end_processing(&mut self, parent: &mut MessageQueue) {
        assert!(self.state == NodeState::ProcessingClauses, "Node {} is not processing", self.id);
        assert!(self.clause_index == self.table.len() - 1, "Node {} is not at the end of the clause", self.id);
        if self.sat_flag {
            self.sat(parent);
        } else {
            self.state = NodeState::Branching;
            self.last_update += 1;
        } 
    }
    fn unsat(&mut self, parent: &mut MessageQueue) {
        if self.speculative_branches.is_empty() {
            self.state = NodeState::AwaitingFork;
        } else {
            self.backtrack(parent);
        }
    }

    fn backtrack(&mut self, parent: &mut MessageQueue) {
        assert!(!self.speculative_branches.is_empty(), "Node {} is backtracking with no branches", self.id);
        self.last_update = self.speculative_branches.pop().expect("No branches to backtrack");
        let msg = Message::SubsitutionQuery {id: self.last_update, assignment: true, reset: true};  // speculative forking always starts with the variable being false so we set to true
        self.send_message(parent, MessageDestination::ClauseTable, msg);
        self.init_processing();
    }

    fn sat(&mut self, network: &mut MessageQueue) {
        self.send_message(network, MessageDestination::Broadcast, Message::Success);
    }

    pub fn recieve_message(&mut self, from: MessageDestination, message: Message) {
        match from {
            MessageDestination::Neighbor(id) => {
                assert!(self.neighbors.contains(&id), "Node {} received message from non-neighbor", self.id);
            },
            MessageDestination::ClauseTable => {
                assert!(self.state == NodeState::ProcessingClauses, "Node {} received message from unexpected source", self.id);
            },
            _ => panic!("Node {} received unexpected message", self.id)
        }
        if self.incoming_message.is_some() {
            // print!("Node {} received message {:?} from {:?} but already has message {:?}", self.id, message, from, msg);
            println!("Node received multiple messages in one cycle");
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
}