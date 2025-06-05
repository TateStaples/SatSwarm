use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Duration;
use bincode::{config, Decode, Encode};
use bincode::config::Configuration;
use crate::structures::microsat;
use crate::structures::microsat::{parse_dimacs, ClauseId, Expression, TraceId};
use crate::structures::node::NodeState;
use crate::structures::testing::{get_test_files, ArchitectureDescription, ProblemDescription, TestLog, TestResult, Topology};
use crate::structures::util_types::{NodeId, Time};

const BINARY_CONFIG: Configuration = config::standard();

#[derive(Encode, Decode, Clone)]
pub struct Trace {
    // data: u64  // 8 bytes per Trace
    unit_props: u16,
    unsat_clause: u16,
    right_child: u32,
}
#[derive(Debug, Clone)]
struct TraceNode {
    branches: Vec<(TraceId, Time, TraceId)>,
    trail: Vec<bool>,
    local_time: Time,
    state: NodeState,
    id: NodeId,
}
impl TraceNode {
    pub fn new(id: NodeId) -> Self {
        Self {
            branches: Vec::new(),
            trail: Vec::new(),
            local_time: 0,
            state: NodeState::Idle,
            id,
        }
    }
}
impl Trace {
    pub fn unsat(unit_props: u16, unsat_clause: ClauseId) -> Self {
        Self { unit_props, unsat_clause, right_child: u32::MAX }
    }
    pub fn sat(unit_props: u16) -> Self {
        Self { unit_props, unsat_clause: u16::MAX, right_child: u32::MAX }
    }
    pub fn branch(unit_props: u16, right_child: usize) -> Self { Self { unit_props, unsat_clause: u16::MAX, right_child: right_child as u32 } }
    pub fn placeholder() -> Self { Self { unit_props: u16::MAX, unsat_clause: u16::MAX, right_child: u32::MAX } }
    #[inline]
    pub fn is_sat(&self) -> bool { self.unsat_clause == u16::MAX && self.right_child == u32::MAX }
    #[inline]
    pub fn is_unsat(&self) -> bool { self.right_child == u32::MAX && !self.is_sat() }
    #[inline]
    pub fn is_branch(&self) -> bool { self.right_child != u32::MAX }
}
impl Debug for Trace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.is_sat() {
            write!(f, "SAT")
        } else if self.is_unsat() {
            write!(f, "UNSAT(unit:{},clz:{})", self.unit_props, self.unsat_clause)
        } else {
            write!(f, "BRANCH(unit:{},right:{})", self.unit_props, self.right_child)
        }
    }
}
/// Save a log of trace information for later analysis
pub fn save_log(log: Vec<Trace>, num_vars: usize, num_clauses: usize, path: PathBuf) {
    let written_obj = (num_vars, num_clauses, log);
    let mut writer = File::create(path.clone()).unwrap();
    bincode::encode_into_std_write(written_obj, &mut writer, BINARY_CONFIG).unwrap();
}
/// Read a log of trace information for analysis
pub fn load_log(path: PathBuf) -> (usize, usize, Vec<Trace>) {
    let mut reader = BufReader::new(File::open(path).unwrap());
    bincode::decode_from_std_read(&mut reader, BINARY_CONFIG).unwrap()
}
/// Structure to control access and adjacency information about the network
#[derive(Clone)]
struct TraceArena {
    nodes: Vec<TraceNode>,
    neighbors: Vec<Vec<NodeId>>,
}
impl TraceArena {
    pub fn new() -> Self {
        TraceArena {
            nodes: Vec::new(),
            neighbors: Vec::new(),
        }
    }

    /// Returns a topology of nodes with connections in the form specified
    pub fn generate(topology: &Topology) -> Self {
        let mut swarm = match topology {
            Topology::Grid(rows, cols) => Self::grid(*rows, *cols),
            Topology::Torus(rows, cols) => Self::torus(*rows, *cols),
            Topology::Dense(num_nodes) => Self::dense(*num_nodes),
        };
        swarm
    }
    pub fn grid(rows: usize, cols: usize) -> Self {
        let mut arena = TraceArena {
            nodes: Vec::with_capacity(rows * cols),
            neighbors: vec![vec![]; rows * cols],
        };
        for i in 0..rows {
            for j in 0..cols {
                let id = arena.nodes.len();
                arena.nodes.push(TraceNode::new(id));
                if i > 0 {
                    arena.add_neighbor(id, id - cols);
                }
                if j > 0 {
                    arena.add_neighbor(id, id - 1);
                }
            }
        }
        arena
    }
    pub fn torus(rows: usize, cols: usize) -> Self {
        let mut arena = TraceArena {
            nodes: Vec::with_capacity(rows * cols),
            neighbors: vec![vec![]; rows * cols],
        };
        for row_index in 0..rows {
            for col_index in 0..cols {
                let id = arena.nodes.len();
                debug_assert_eq!(id, row_index * cols + col_index, "Node id {} does not match expected id {}", id, row_index * cols + col_index);
                arena.nodes.push(TraceNode::new(id));
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
        arena
    }
    pub fn dense(num_nodes: usize) -> Self {
        let mut arena = TraceArena {
            nodes: Vec::with_capacity(num_nodes),
            neighbors: vec![vec![]; num_nodes],
        };
        for id in 0..num_nodes {
            arena.nodes.push(TraceNode::new(id));
        }
        for i in 0..num_nodes {
            for j in (i + 1)..num_nodes {
                arena.add_neighbor(i, j);
            }
        }
        arena
    }

    pub fn get_node(&self, id: NodeId) -> &TraceNode { self.nodes.get(id).expect("Node not found") }
    pub fn get_node_mut(&mut self, id: NodeId) -> &mut TraceNode { self.nodes.get_mut(id).expect("Node not found") }
    pub fn get_node_opt(&self, id: NodeId) -> Option<&TraceNode> { self.nodes.get(id) }
    pub fn get_node_mut_opt(&mut self, id: NodeId) -> Option<&mut TraceNode> { self.nodes.get_mut(id) }

    pub fn add_neighbor(&mut self, node_id: NodeId, neighbor_id: NodeId) {
        self.neighbors[node_id].push(neighbor_id);
        self.neighbors[neighbor_id].push(node_id);
    }

    pub fn remove_neighbor(&mut self, node_id: NodeId, neighbor_id: NodeId) {
        self.neighbors[node_id].remove(neighbor_id);
        self.neighbors[neighbor_id].remove(node_id);
    }

    pub fn get_neighbors(&self, id: NodeId) -> &Vec<NodeId> {
        &self.neighbors[id]
    }
    pub fn get_neighbors_mut(&mut self, id: NodeId) -> &mut Vec<NodeId> {
        &mut self.neighbors[id]
    }
}
struct TraceFork {
    fork_time: Time,
    trail: Vec<bool>,
    new_branch: TraceId,
}
pub fn test_traces(test_path: String, config: ArchitectureDescription) {
    let arena = TraceArena::generate(&config.topology);
    let save_path = TestLog::create_log_path();
    if let Some(files) = get_test_files(&test_path) {
        for trace_file in files {
            println!("Running test: {:?}", trace_file.clone());
            let test_name = trace_file.file_stem().unwrap().to_str().unwrap()
                    .split("trace_of_").last().unwrap().to_string();
            let (num_vars, num_clauses, mut log) = load_log(trace_file.clone());
            let initial_log_len = log.len();
            let source_path = get_test_files("tests").unwrap().into_iter().filter(|x| x.file_name().unwrap().to_str().unwrap().contains(&test_name)).last().unwrap();
            println!("Source path: {:?}", source_path);
            let mut cnf = parse_dimacs(source_path);
            cnf.optimize();
            let num_vars = cnf.num_vars(); let num_clauses = cnf.num_clauses();
            debug_assert!(cnf.clauses.iter().filter(|clause| clause.enabled).all(|x| !x.variables.is_empty()));
            let problem_description = ProblemDescription {
                num_vars,
                num_clauses,
                test_name,
            };
            let start_time = std::time::Instant::now();
            let test_result = run(arena.clone(), &mut log, cnf, config.clone());
            println!("Test took {} s", start_time.elapsed().as_secs_f64());
            if log.len() > initial_log_len {
                assert!(test_result.simulated_result);
                println!("Log length increased! {}->{}", initial_log_len, log.len());
                save_log(log.clone(), num_vars, num_clauses, trace_file);
            }
            let test_log = TestLog {
                problem_description,
                config: config.clone(),
                test_result,
                expected_result: false,
                minisat_speed: Duration::ZERO,
            };
            test_log.save(save_path.clone());
        }
    } else {
        println!("No test files found.");
    }

}
fn run(mut arena: TraceArena, log: &mut Vec<Trace>, cnf: Expression, config: ArchitectureDescription) -> TestResult {
    println!("Running with size {}", log.len());
    debug_assert!(cnf.clauses.iter().filter(|clause| clause.enabled).all(|x| !x.variables.is_empty()));
    let num_clauses = cnf.clauses.len();
    // Initialization
    let mut results = TestResult {
        simulated_result: false,
        simulated_cycles: 0,
        cycles_busy: 0,
        cycles_idle: 0,
    };
    let mut activated = BinaryHeap::from(arena.nodes.iter().map(|n| Reverse((n.local_time, n.id))).collect::<Vec<_>>());


    let activate_node = |node: &mut TraceNode, log: &Vec<Trace>| {
        node.state = NodeState::Busy;
        propagate(node, 0, log, &config, num_clauses);
    };

    fn propagate(node: &mut TraceNode, id: TraceId, log: &Vec<Trace>, config: &ArchitectureDescription, num_clauses: usize) {
        debug_assert!(id < log.len());
        let trace = &log[id];
        node.local_time += config.decision_delay + div_up(num_clauses as Time, config.clause_per_eval as Time) * (config.cycles_per_eval as Time) * (trace.unit_props as Time);
        if trace.is_branch() {
            debug_assert!(id >= node.branches.last().map(|x|x.0).unwrap_or(0));
            node.branches.push((id, node.local_time, trace.right_child as usize));
            node.trail.push(false);
            propagate(node, id + 1, log, config, num_clauses);  // left branch
        } else if trace.is_sat() {
            node.state = NodeState::SAT;
        } else {
            debug_assert!(trace.is_unsat());
            let unsat_clause = trace.unsat_clause;
            node.local_time += div_up(unsat_clause as Time,config.clause_per_eval as Time) * (config.cycles_per_eval as Time);
        }
    }

    let retry = |node: &mut TraceNode, log: &mut Vec<Trace>| {
        debug_assert!(node.state == NodeState::Busy);
        while let Some((source_id, branch_time, mut branch_id)) = node.branches.pop() {
            node.trail.pop();
            if branch_id == 1 { continue; }  // stolen work
            if branch_id == 0 {  // Uncompleted (should very rarely happen
                branch_id = log.len();
                log[source_id].right_child = branch_id as u32;
                node.trail.push(true);
                let mut now = cnf.clone();
                // NOTE: this isn't thoroughly tested because it is a rare case
                now.follow_trail(&node.trail);
                now.dpll(log);
            }
            node.local_time += div_up(num_clauses as Time, config.clause_per_eval as Time) * (config.cycles_per_eval as Time);
            propagate(node, branch_id, log, &config, num_clauses);
            return;
        }
        debug_assert!(node.branches.is_empty());
        node.state = NodeState::Idle;
    };

    let receive_fork = |node: &mut TraceNode, fork: TraceFork, log: &Vec<Trace>| {
        debug_assert!(node.branches.is_empty());
        let TraceFork { fork_time, new_branch, trail } = fork;
        debug_assert!(fork_time > node.local_time);
        debug_assert!(new_branch > 1);
        node.trail = trail;
        node.local_time = fork_time;
        node.branches.clear();
        node.state = NodeState::Busy;
        propagate(node, new_branch, log, &config, num_clauses);
    };
    #[inline]
    fn div_up(a: Time, b: Time) -> Time { (a + (b - 1)) / b }

    let retire_node = |arena: &TraceArena, id: NodeId, busy: &mut i32, activated: &mut BinaryHeap<Reverse<(Time, NodeId)>>| {
        let node = arena.get_node(id);
        match node.state {
            NodeState::Busy | NodeState::SAT => {    // Busy nodes should resume once we've caught back to their local time
                *busy += 1;
                activated.push(Reverse((node.local_time, id)));
            }
            NodeState::Idle => {
                activated.push(Reverse((node.local_time, id)));
            }
        }
    };

    let create_fork = |arena: &mut TraceArena, id: NodeId, local_time: Time, log: &mut Vec<Trace>| -> Option<TraceFork> {
        let neighbors = arena.get_neighbors(id);
        let search = neighbors.iter()
            .map(|n| {
                let neighbor = arena.get_node(*n);
                let best_idx = neighbor.branches.iter()
                    .position(|(_, branch_time, new_branch)| *new_branch != 1  // FIXME
                        && *branch_time >= local_time);
                if let Some(best_idx) = best_idx {
                    Some((*n, best_idx, neighbor.branches[best_idx]))
                } else {
                    None
                }
            })
            .filter(|search| search.is_some())
            .map(|search| search.unwrap())
            .min_by(|(_, _, (_, a, _)), (_, _, (_, b, _))| a.cmp(&b));
        if let Some((neighbor_id, best_idx, (source_id, time, mut new_branch))) = search {
            let neighbor = arena.get_node_mut(neighbor_id);
            let (_, _, partner_branch) = &mut neighbor.branches[best_idx];
            *partner_branch = 1;  // mark as stolen
            let fork_time = time + config.fork_delay;
            let trail_idx = best_idx+neighbor.trail.len()-neighbor.branches.len();
            let mut new_trail = neighbor.trail[..trail_idx].to_vec();
            new_trail.push(!neighbor.trail[trail_idx]);
            if new_branch == 0 {
                new_branch = log.len();
                log[source_id].right_child = new_branch as u32;
                let mut now = cnf.clone();
                now.follow_trail(&new_trail);
                print!("Expanding trace on {:?}...", new_trail);
                now.dpll(log);
                println!(" Done.");
                assert!(log.len() < u32::MAX as usize);
            }
            Some(TraceFork {
                new_branch,
                trail: new_trail,
                fork_time,
            })
        } else {
            None
        }
    };

    // Kickstart the reaction
    let mut busy_count = 0;
    let Reverse((_, start_point)) = activated.pop().unwrap();
    let start_node = arena.get_node_mut(start_point);
    activate_node(start_node, log);
    results.cycles_busy += start_node.local_time;
    retire_node(&arena, start_point, &mut busy_count, &mut activated);

    // Main Event loop
    while busy_count > 0 {
        let Reverse((local_time, id)) = activated.pop().unwrap();
        let node = arena.get_node(id);
        // Perform action depending on the state
        match node.state {
            NodeState::Busy => {  // maybe rename this
                busy_count -= 1;
                let node = arena.get_node_mut(id);
                let start_time = node.local_time;
                retry(node, log);
                results.cycles_busy += node.local_time - start_time;
            }
            NodeState::Idle => {  // Look to neighbors for fork
                if let Some(fork) = create_fork(&mut arena, id, local_time, log) {
                    results.cycles_idle += fork.fork_time - local_time;
                    let node = arena.get_node_mut(id);
                    debug_assert!(&fork.fork_time > &node.local_time, "Fork received before done!");
                    let fork_time = fork.fork_time;
                    receive_fork(node, fork, log);
                    results.cycles_busy += node.local_time - fork_time;
                } else {    // become sleepy as no neighbors have anything at this time
                    let neighbors = arena.get_neighbors(id);
                    let min_neighbor_time = neighbors.iter().map(|n| arena.get_node(*n).local_time).min().unwrap();
                    let node = arena.get_node_mut(id);
                    results.cycles_idle += min_neighbor_time + config.fork_delay - local_time;
                    node.local_time = min_neighbor_time + config.fork_delay;
                }
            }
            NodeState::SAT => {
                let node = arena.get_node_mut(id);
                println!("Found SAT! {:?}, {}", node, local_time);
                results.simulated_result = true;
                results.simulated_cycles = local_time;
                for node in arena.nodes.iter() {
                    // Remove work from the future
                    results.cycles_busy -= (node.local_time - local_time).max(0);
                }
                break;  // we are done
            }
        }

        // figure out what to do with the node next
        retire_node(&arena, id, &mut busy_count, &mut activated)
    }
    if results.simulated_cycles == 0 {
        results.simulated_cycles = arena.nodes.iter().map(|n| n.local_time).max().unwrap();
        results.simulated_result = false;
    }
    println!("Results: {:?}", results);
    results
}