use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::BufReader;
use std::process::exit;
use bincode::{config, Decode, Encode};
use bincode::config::Configuration;
use crate::structures::microsat::{ClauseId, TraceId};
use crate::structures::node::NodeState;
use crate::structures::testing::{TestResult, Topology};
use crate::structures::util_types::{NodeId, Time, DEBUG_PRINT};

const BINARY_CONFIG: Configuration = config::standard();

#[derive(Encode, Decode)]
pub struct Trace {
    // data: u64  // 8 bytes per Trace
    unit_props: u16,
    unsat_clause: u16,
    right_child: u32
}
impl Trace {
    pub fn unsat(unit_props: u16, unsat_clause: ClauseId) -> Self {
        Self { unit_props, unsat_clause, right_child: u32::MAX }
    }
    pub fn sat(unit_props: u16) -> Self {
        Self { unit_props, unsat_clause: u16::MAX, right_child: u32::MAX }
    }
    pub fn branch(unit_props: u16, right_child: usize) -> Self { Self { unit_props, unsat_clause: u16::MAX, right_child: right_child as u32} }
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
pub fn save_log(log: Vec<Trace>, path: String) {
    let mut writer = File::create(path).unwrap();
    bincode::encode_into_std_write(log, &mut writer, BINARY_CONFIG).unwrap();
}
fn load_log(path: String) -> Vec<Trace> {
    let mut reader = BufReader::new(File::open(path).unwrap());
    bincode::decode_from_std_read(&mut reader, BINARY_CONFIG).unwrap()
}

pub struct ArchitectureDescription {
    topology: Topology,
    decision_delay: Time,
    fork_delay: Time,
    clause_per_eval: usize,
    cycles_per_eval: Time,
}

struct TraceNode {
    branches: Vec<(Time, TraceId)>,
    local_time: Time,
    state: NodeState,
    id: NodeId,
}

impl Eq for TraceNode {}
impl PartialEq<Self> for TraceNode {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id 
    }
}

impl PartialOrd<Self> for TraceNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.local_time.cmp(&other.local_time).reverse())
    }
}

impl Ord for TraceNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

fn num_nodes(topology: &Topology) -> usize {
    match topology { 
        Topology::Dense(s) => *s,
        Topology::Grid(x, y) => *x *  *y,
        Topology::Torus(x, y) => *x *  *y,
    }
}


struct TraceFork {
    fork_time: Time,
    new_branch: TraceId
}
impl TraceFork {
    fn activate() {
        
    }
}
fn find_fork(node: &TraceNode, topology: &Topology) -> Option<TraceFork> {
    todo!()
}

fn find_neighbors(node: &TraceNode, topology: &Topology) -> Vec<NodeId> {
    todo!()
}
// pub fn test(test: ArchitectureDescription) {
//     let mut results = TestResult {
//         simulated_result: false,
//         simulated_cycles: 0,
//         cycles_busy: 0,
//         cycles_idle: 0,
//     };
//     // let trace_log_path = todo!();
//     // let trace_path = todo!();
//     let log = load_log(trace_log_path);
//     let node_counts = num_nodes(&test.topology);
//     let nodes = Vec::from_iter((0..node_counts).map(|i| TraceNode {
//         branches: Vec::new(), local_time: 0, state: NodeState::Idle, id: i
//     }));
//     
//     let start_node = &mut nodes[0];
//     start_node.activate();
//     results.cycles_busy += start_node.local_time;
// 
//     // BFS the forks first and then do heap
//     let mut priority_queue = BinaryHeap::<TraceNode>::from(nodes);
//     let mut busy_count = 1;
//     // Main Event loop
//     while busy_count > 0 {
//         let node = priority_queue.pop().unwrap();
//         // Perform action depending on the state
//         match node.state {
//             NodeState::Busy => {  // maybe rename this
//                 busy_count -= 1;
//                 let start_time = node.local_time;
//                 node.retry();
//                 results.cycles_busy += node.local_time - start_time;
//             }
//             NodeState::Idle => {  // Look to neighbors for fork
//                 if let Some(fork) = find_fork(&node, &test.topology) {
//                     results.cycles_idle += fork.fork_time - node.local_time;
//                     let fork_time = fork.fork_time;
//                     node.receive_fork(fork);
//                     results.cycles_busy += node.local_time - fork_time;
//                 } else {    // become sleepy as no neighbors have anything at this time
//                     let min_neighbor_time = find_neighbors(&node, &test.topology).iter().map(|n| self.arena.get_node(*n).local_time).min().unwrap();
//                     let node = self.arena.get_node_mut(id);
//                     self.results.cycles_idle += min_neighbor_time + self.fork_delay - local_time;
//                     node.local_time = min_neighbor_time + self.fork_delay;
//                 }
//             }
//             NodeState::SAT => {
//                 let node = self.arena.get_node_mut(id);
//                 println!("Found SAT! {:?}, {}", node, local_time);
//                 // println!("Node Assignments: {:?}", node.assignments.iter().enumerate().collect::<Vec<_>>());
//                 // node.print_model();
//                 // assert!(!node.problem_unsat(), "Invalid SAT reached!");
//                 results.simulated_result = true;
//                 self.results.simulated_cycles = local_time;
//                 for node in self.arena.nodes.iter() {
//                     // Remove work from the future
//                     results.cycles_busy -= (node.local_time - local_time).max(0);
//                 }
//             }
//         }
//         // TODO: add back in
//     }
//     
//     
// }