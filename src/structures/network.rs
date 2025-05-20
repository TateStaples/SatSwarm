use std::cmp::PartialEq;
use std::collections::{BinaryHeap, HashMap};

use crate::{structures::clause_table::{TermState}, TestConfig};
use crate::structures::clause_table::ProblemState;
use crate::structures::node::{AssignmentCause, Fork, NodeState};
use crate::structures::testing::{TestResult, Topology};
use super::{clause_table::ClauseTable, node::Node, util_types::{NodeId, VarId, DEBUG_PRINT}};

/// Structure to control access and adjacency information about the network
struct Arena {
    nodes: Vec<Node>,
    neigbors: Vec<Vec<NodeId>>,
}
impl Arena {
    pub fn new() -> Self {
        Arena {
            nodes: Vec::new(),
            neigbors: Vec::new()
        }
    }

    pub fn from_nodes(nodes: Vec<Node>) -> Self {
        Arena {
            nodes,
            neigbors: Vec::new()
        }
    }


    pub fn get_node(&self, id: NodeId) -> &Node {self.nodes.get(id).expect("Node not found")}
    pub fn get_node_mut(&mut self, id: NodeId) -> &mut Node {self.nodes.get_mut(id).expect("Node not found")}
    pub fn get_node_opt(&self, id: NodeId) -> Option<&Node> {self.nodes.get(id)}
    pub fn get_node_mut_opt(&mut self, id: NodeId) -> Option<&mut Node> {self.nodes.get_mut(id)}

    pub fn add_neighbor(&mut self, node_id: NodeId, neighbor_id: NodeId) {
        self.neigbors[node_id].push(neighbor_id);
        self.neigbors[neighbor_id].push(node_id);
    }

    pub fn remove_neighbor(&mut self, node_id: NodeId, neighbor_id: NodeId) {
        self.neigbors[node_id].remove(neighbor_id);
        self.neigbors[neighbor_id].remove(node_id);
    }

    pub fn get_neighbors(&self, id: NodeId) -> &Vec<NodeId> {
        &self.neigbors[id]
    }
    pub fn get_neighbors_mut(&mut self, id: NodeId) -> &mut Vec<NodeId> {
        &mut self.neigbors[id]
    }
}

pub struct Network {
    arena: Arena,
    fork_delay: u64,
    results: TestResult
}


impl Network {
    // ----- Constructors ----- // 
    // TODO: add documentation to the constructors
    fn build(arena: Arena) -> Network {
        Network {
            arena,
            fork_delay: 0, // TODO: make this better later for timing purposes
            results: Default::default()
        }
    }
    pub fn _blank() -> Network {
        Network::build(Arena::new())
    }
    pub fn generate(clause_table: ClauseTable, config: &TestConfig) -> Network {
        let mut swarm = match config.topology {
            Topology::Grid(rows, cols) => Network::grid(clause_table, rows, cols, config.node_bandwidth),
            Topology::Torus(rows, cols) => Network::torus(clause_table, rows, cols, config.node_bandwidth),
            Topology::Dense(num_nodes) => Network::dense(clause_table, num_nodes, config.node_bandwidth),
        };
        // swarm.messages.set_bandwidth(config.node_bandwidth);
        swarm
    }
    pub fn grid(clause_table: ClauseTable, rows: usize, cols: usize, node_bandwidth: usize) -> Network {
        let mut arena = Arena {
            nodes: Vec::with_capacity(rows * cols),
            neigbors: vec![vec![]; rows * cols]
        };
        for i in 0..rows {
            for j in 0..cols {
                let id = arena.nodes.len();
                arena.nodes.push(Node::new(id, clause_table.clone(), node_bandwidth,1));
                if i > 0 {
                    arena.add_neighbor(id, id - cols);
                }
                if j > 0 {
                    arena.add_neighbor(id, id - 1);
                }
            }
        }
        Network::build(arena)
    }
    pub fn torus(clause_table: ClauseTable, rows: usize, cols: usize, node_bandwidth: usize) -> Network {
        let mut arena = Arena {
            nodes: Vec::with_capacity(rows * cols),
            neigbors: vec![vec![]; rows * cols]
        };
        for row_index in 0..rows {
            for col_index in 0..cols {
                let id = arena.nodes.len();
                assert!(id == row_index * cols + col_index, "Node id {} does not match expected id {}", id, row_index * cols + col_index);
                arena.nodes.push(Node::new(id, clause_table.clone(), node_bandwidth, 1));
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
        Network::build(arena)
    }
    pub fn dense(clause_table: ClauseTable, num_nodes: usize, node_bandwidth: usize) -> Network {
        let mut arena = Arena {
            nodes: Vec::with_capacity(num_nodes),
            neigbors: vec![vec![]; num_nodes]
        };
        for id in 0..num_nodes {
            arena.nodes.push(Node::new(id, clause_table.clone(), node_bandwidth, 1));
        }
        for i in 0..num_nodes {
            for j in (i + 1)..num_nodes {
                arena.add_neighbor(i, j);
            }
        }
        Network::build(arena)
    }
    /// The event-driven simulator. Runs all nodes until they hit an UNSAT or SAT state
    /// Nodes are activated by earliest local time (filtering out un-active ones to prevent livelock)
    /// UNSAT will backtrack starting from the latest speculative decisions
    /// Idle nodes still earliest neighboring speculative decision
    fn run_event_loop(&mut self) {
        self.results = TestResult {  // confirm that we start with the correct default values
            simulated_result: false,
            simulated_cycles: 0,
            cycles_busy: 0,
            cycles_idle: 0,
        };
        let activated: Vec<(u64, NodeId)> = self.arena.nodes.iter()
            .filter(|node| node.state != NodeState::Sleep)
            .map(|n| (n.local_time, n.id))
            .collect();
        let mut activated = BinaryHeap::from(activated);

        // Main Event loop
        while activated.len() > 0 {
            let (local_time, id) = activated.pop().unwrap();
            let current_state = &self.arena.get_node(id).state;

            let node = self.arena.get_node(id);
            // Perform action depending on the state
            match node.state {
                NodeState::Busy => {
                    let node = self.arena.get_node_mut(id);
                    node.retry();
                }
                NodeState::Idle => {  // Look to neighbors for fork
                    if let Some(fork) = self.create_fork(id, local_time) {
                        let node = self.arena.get_node_mut(id);
                        node.receive_fork(fork);
                    } else {    // become sleepy as no neighbors have anything at this time
                        let node = self.arena.get_node_mut(id);
                        node.state = NodeState::Sleep;
                    }
                }
                NodeState::SAT => {
                    self.results.simulated_result = true;
                    self.results.simulated_cycles = local_time;
                    break;  // we are done
                }
                NodeState::Sleep => unreachable!()
            }

            // figure out what to do with the node next
            let node = self.arena.get_node(id);
            match node.state {
                NodeState::Sleep => {}  // Do nothing, don't check on this until something wakes it up
                NodeState::Busy => {    // Busy nodes should resume once we've caught back to their local time
                    activated.push((node.local_time, id));

                    // Back sure to activate the nodes neighboring
                    let neighbors: Vec<NodeId> = self.arena.get_neighbors(id).iter().copied().collect();
                    for &neighbor_id in &neighbors {
                        let neighbor_state = &self.arena.get_node(neighbor_id).state;
                        if *neighbor_state == NodeState::Sleep {
                            let neighbor = self.arena.get_node_mut(neighbor_id);
                            neighbor.state = NodeState::Idle;
                        }
                    }
                }
                _ => {}
            }
        }
        // Problem is UNSAT (default result) as nothing through SAT before problem terminated
    }

    fn create_fork(&self, id: NodeId, local_time: u64) -> Option<Fork> {
        // TODO: also modify the stolen assignment from Speculative cause to fork cause
        // TODO: also add a time into the fork
        let neighbors = self.arena.get_neighbors(id);
        let search = neighbors.iter()
            .map(|n| {
                let neighbor = self.arena.get_node(*n);
                let best_idx = neighbor.assignment_history.iter()
                    .position(|assignment| assignment.cause == AssignmentCause::Speculative
                            && assignment.time >= local_time);
                if let Some(best_idx) = best_idx {
                    Some((*n, best_idx, &mut neighbor.assignment_history[best_idx]))
                } else {
                    None
                }
        })
            .filter(|assignment| assignment.is_some())
            .map(|assignment| assignment.unwrap())
            .min_by(|(_,_,a), (_,_,b)|
                a.time.cmp(&b.time));
        if let Some((neighbor_id, best_idx, assignment)) = search {
            assignment.cause = AssignmentCause::Fork;
            
        } else {
            None
        }
    }
    pub fn test_satisfiability(&mut self) -> TestResult {
        { 
            self.arena.get_node_mut(0).activate(); 
        }
        self.run_event_loop();
        // println!("Result: {:?}", self.results);
        self.results.clone()
    }


    fn recover_model(&self, id: NodeId) -> HashMap<VarId, bool> {
        let node = self.arena.get_node(id);
        node.assignments.clone()
            .into_iter()
            .enumerate()
            .map(|(idx, a)| (idx as VarId, a.unwrap_or(false)))
            .collect()

    }
}
