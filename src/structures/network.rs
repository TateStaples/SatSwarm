use std::cmp::{PartialEq, Reverse};
use std::collections::{BinaryHeap, HashMap};
use std::process::exit;
use crate::{structures::clause_table::{TermState}, TestConfig};
use crate::structures::clause_table::ProblemState;
use crate::structures::node::{AssignmentCause, Fork, NodeState, VariableAssignment};
use crate::structures::testing::{TestResult, Topology};
use crate::structures::util_types::Time;
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
    fork_delay: Time,
    results: TestResult
}


impl Network {
    // ----- Constructors ----- // 
    // TODO: add documentation to the constructors
    fn build(arena: Arena) -> Network {
        Network {
            arena,
            fork_delay: 1, // TODO: make this better later for timing purposes
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
    fn run_event_loop(&mut self, start_point: NodeId) {
        if DEBUG_PRINT {
            println!("Running event loop");
        }
        // Initialization
        self.results = TestResult {
            simulated_result: false,
            simulated_cycles: 0,
            cycles_busy: 0,
            cycles_idle: 0,
        };
        let mut activated = BinaryHeap::from(self.arena.nodes.iter().map(|n| Reverse((n.local_time, n.id))).collect::<Vec<_>>());

        // Kickstart the reaction
        let mut busy_count = 0;
        let Reverse((_, start_node)) = activated.pop().unwrap();
        let start_node = self.arena.get_node_mut(start_node);
        start_node.activate();
        self.results.cycles_busy += start_node.local_time;
        self.retire_node(start_point, &mut busy_count, &mut activated);

        // Main Event loop
        while busy_count > 0 {
            let Reverse((local_time, id)) = activated.pop().unwrap();
            if DEBUG_PRINT {
                println!("Activating node {}", id);
            }

            let node = self.arena.get_node(id);
            // Perform action depending on the state
            match node.state {
                NodeState::Busy => {  // maybe rename this
                    busy_count -= 1;
                    let node = self.arena.get_node_mut(id);
                    let start_time = node.local_time;
                    node.retry();
                    self.results.cycles_busy += node.local_time - start_time;
                }
                NodeState::Idle => {  // Look to neighbors for fork
                    if let Some(fork) = self.create_fork(id, local_time) {
                        self.results.cycles_idle += fork.fork_time - local_time;
                        let node = self.arena.get_node_mut(id);
                        assert!(&fork.fork_time > &node.local_time, "Fork received before done!");
                        node.receive_fork(fork);
                        self.results.cycles_busy += node.local_time - local_time;
                    } else {    // become sleepy as no neighbors have anything at this time
                        let neighbors = self.arena.get_neighbors(id);
                        let min_neighbor_time = neighbors.iter().map(|n| self.arena.get_node(*n).local_time).min().unwrap();
                        let node = self.arena.get_node_mut(id);
                        self.results.cycles_idle += min_neighbor_time + self.fork_delay - local_time;
                        node.local_time = min_neighbor_time + self.fork_delay;
                    }
                }
                NodeState::SAT => {
                    let node = self.arena.get_node_mut(id);
                    println!("Found SAT! {:?}, {}", node, local_time);
                    // println!("Node Assignments: {:?}", node.assignments.iter().enumerate().collect::<Vec<_>>());
                    // node.print_model();
                    // assert!(!node.problem_unsat(), "Invalid SAT reached!");
                    self.results.simulated_result = true;
                    self.results.simulated_cycles = local_time;
                    for node in self.arena.nodes.iter() {
                        // Remove work from the future
                        self.results.cycles_busy -= (node.local_time - local_time).max(0);
                    }
                    break;  // we are done
                }
            }

            // figure out what to do with the node next
            self.retire_node(id, &mut busy_count, &mut activated)
        }
        if self.results.simulated_cycles == 0 {
            exit(1);
        }
        // Problem is UNSAT (default result) as nothing through SAT before problem terminated
    }

    fn retire_node(&mut self, id: NodeId, busy: &mut i32, activated: &mut BinaryHeap<Reverse<(Time, NodeId)>>) {
        let node = self.arena.get_node(id);
        match node.state {
            NodeState::Busy | NodeState::SAT => {    // Busy nodes should resume once we've caught back to their local time
                *busy += 1;
                activated.push(Reverse((node.local_time, id)));
            }
            NodeState::Idle => {
                activated.push(Reverse((node.local_time, id)));
            }
        }
    }

    fn create_fork(&mut self, id: NodeId, local_time: Time) -> Option<Fork> {
        let neighbors = self.arena.get_neighbors(id);
        let search = neighbors.iter()
            .map(|n| {
                let neighbor = self.arena.get_node(*n);
                let best_idx = neighbor.assignment_history.iter()
                    .position(|assignment| assignment.cause == AssignmentCause::Speculative
                            && assignment.time >= local_time);
                if let Some(best_idx) = best_idx {
                    Some((*n, best_idx, &neighbor.assignment_history[best_idx]))
                } else {
                    None
                }
        })
            .filter(|search| search.is_some())
            .map(|search| search.unwrap())
            .min_by(|(_,_,a), (_,_,b)|
                a.time.cmp(&b.time));
        if let Some((neighbor_id, best_idx, _)) = search {
            let neighbor = self.arena.get_node_mut(neighbor_id);
            let mut variable_assignments = neighbor.assignments.clone();
            for VariableAssignment { var_id, .. } in &neighbor.assignment_history[best_idx+1..] {
                variable_assignments[*var_id as usize] = None;
            }
            let assignment = &mut neighbor.assignment_history[best_idx];
            assignment.cause = AssignmentCause::Fork;
            let fork_time = assignment.time + self.fork_delay;
            variable_assignments[assignment.var_id as usize] = Some(!assignment.assignment);
            Some(Fork{
                variable_assignments,
                fork_time
            })
        } else {
            None
        }
    }
    pub fn test_satisfiability(&mut self) -> TestResult {
        if DEBUG_PRINT {
            println!("Testing satisfiability");
        }
        self.run_event_loop(0);
        println!("Result: {:?}", self.results);
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
