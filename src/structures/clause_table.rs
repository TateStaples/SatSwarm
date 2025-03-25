use core::{net, num};
use std::collections::HashMap;
use std::{collections::VecDeque, io::BufRead, path::PathBuf}; // Import VecDeque for FIFO queue
use rand::Rng; // Import rand for random number generation

use crate::{get_clock, DEBUG_PRINT};

use super::node::{CNFState, ClauseState, NodeId, TermState, VarId, CLAUSE_LENGTH}; // Import Network struct
use super::message::{Message, MessageDestination, MessageQueue, TermUpdate}; // Import Message struct
struct Query {
    source: NodeId,
    var: VarId,
    set: bool,
    reset: bool,
    updates_left: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Term {
    pub var: VarId,
    pub negated: bool,
}
pub struct ClauseTable {
    pub clause_table: Vec<[Term; CLAUSE_LENGTH]>, // 2D Vec to store the table of clauses
    // TODO: Shaan, I removed the clock because it was unused
    pub num_clauses: usize,           // Number of clauses in the table
    pub num_vars: usize,              // Number of variables in the table

    query_buffer: VecDeque<Query>, // FIFO queue to hold incoming queries
    // Hey, Shaan just added in queries into the struct because it seemed simpler than having a separate hashmap
    inflight_queries: HashMap<NodeId, Query>, // hashmap query: VarId -> updates_left_to_process: u64
}

impl ClauseTable {
    // Creates a new ClauseTable
    pub fn _dummy() -> Self {
        let num_clauses = 10; // Number of clauses in the table
        Self {
            clause_table: vec![[Default::default(); CLAUSE_LENGTH]; num_clauses as usize], // Initialize the clause table with 0s
            num_clauses: num_clauses, // Initialize the number of clauses
            query_buffer: VecDeque::new(), // Initialize an empty FIFO queue
            inflight_queries: HashMap::new(), // Initialize an empty hashmap
            num_vars: 1,
        }
    }

    pub fn random(num_clauses: usize, num_vars: u8) -> Self {
        let mut clause_table = Vec::with_capacity(num_clauses);
        for _ in 0..num_clauses {
            let mut clause = [Term{var: 0, negated: false}; CLAUSE_LENGTH];
            for i in 0..CLAUSE_LENGTH {
                let var = rand::random::<u8>() % num_vars as u8;
                let negated = rand::random::<bool>();
                clause[i] = Term{var, negated};
            }
            clause_table.push(clause);
        }
        clause_table.push([Term{var: 0, negated: true}; CLAUSE_LENGTH]);  // Add a dummy clause to the end to make var 0 false
        // clause_table.push([Term{var: 0, negated: false}; CLAUSE_LENGTH]);  // Add a dummy clause to the end to make var 0 true (contradiction)
        let num_clauses = clause_table.len();
        Self {
            clause_table,
            num_clauses,
            num_vars: (num_vars as usize),
            query_buffer: VecDeque::new(),
            inflight_queries: HashMap::new(),
        }
    }

    pub fn load_file(file: PathBuf) -> (Self, bool) {
        // Load a file and return a new ClauseTable with expected SAT result
        /* Example File Format                                  (0 is the end of the clause)
        c
        c SAT instance in DIMACS CNF input format.
        c
        p cnf 100 286                                           p cnf <num_vars> <num_clauses>
        80  -39  -21  0                                         <var1> <var2> ... <varN> 0                   
        -58  25  23  0
        -88  55  -42  0
        -71  -49  46  0
         */
        let mut num_clauses = 0;
        let sat = !file.to_string_lossy().to_lowercase().contains("unsat");
        let mut clauses = Vec::new();
        let mut var_count = 0;
        let file = std::fs::File::open(file).unwrap();
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line.unwrap();
            // println!("{}", line);
            let mut clause = [Term{var: 0, negated: false}; CLAUSE_LENGTH];
            let mut clause_end = false;
            if line.starts_with("p cnf") {  // Parse the number of variables and clauses *header*
                let mut parts = line.split_whitespace();
                // println!("{:?}", parts.clone().collect::<Vec<&str>>());
                parts.next(); // Skip "p"
                parts.next(); // Skip "cnf"
                var_count = parts.next().unwrap().parse().unwrap();
                assert!(var_count < u8::MAX as i32, "Too many variables for u8");
                num_clauses = parts.next().unwrap().parse().unwrap();
                clauses = Vec::with_capacity(num_clauses);
            } else if line.starts_with("c") {  // Skip comments
                continue;
            } else if line.starts_with("%") {  // end this file
                break;
            } else {
                let parts = line.split_whitespace();
                for (term_index, part) in parts.enumerate() {
                    assert!(!clause_end, "Clause has already ended");
                    let num: i32 = part.parse().unwrap();
                    if num == 0 {
                        clause_end = true;
                        assert!(term_index <= CLAUSE_LENGTH, "Only 3SAT is supported");
                        for i in term_index..CLAUSE_LENGTH {
                            clause[i] = Term{var: 0, negated: false};  // Var 0 is always false
                        }
                    } else {
                        assert!(num.abs() < u8::MAX as i32, "Too many variables for u8");
                        clause[term_index] = Term{var: num.abs() as u8, negated: num < 0};  // want to 0 index the variables
                    }
                }
            }
            if clause_end {
                clauses.push(clause);
            }
        }
        if num_clauses < 10 {
            println!("Clauses: {:?}, expected_num_clauses: {}, expected_sat: {}, expected_vars: {}", clauses, num_clauses, sat, var_count);
        }
        assert!(clauses.len() == num_clauses, "Number of clauses does not match header");
        clauses.push([Term{var: 0, negated: true}; CLAUSE_LENGTH]);  // Add a dummy clause to the end to make var 0 false
        assert!(clauses.iter().map(|c| c.iter().map(|t| t.var).max().unwrap()).max().unwrap() == var_count as u8, "Variable count does not match header");
        let num_clauses = clauses.len();
        let s = Self {
            clause_table: clauses,
            num_clauses: num_clauses,
            num_vars: (var_count+1) as usize,
            query_buffer: VecDeque::new(),
            inflight_queries: HashMap::new(),
        };

        (s, sat)
    }
    
    pub fn number_of_vars(&self) -> usize {
        self.clause_table.iter().map(|c| c.iter().map(|t| t.var).max().unwrap()).max().unwrap() as usize
    }
    // Updates the ClauseTable
    pub fn clock_update(&mut self, network: &mut MessageQueue) {
        
        // try to see if any new queries have been added to the query buffer
        if !self.query_buffer.is_empty() {
            let query = self.query_buffer.pop_front().unwrap(); // Get the first query from the queue
            if query.var >= self.num_vars as u8 {
                network.start_message(MessageDestination::ClauseTable, MessageDestination::Neighbor(query.source), Message::VariableNotFound);
            } else {
                self.inflight_queries.insert(query.source, query); // Add the query to the inflight queries (we use HashMap to overwrite any existing queries to this core)  TODO: ask shaan if this is reasonable
            }
        }
        if DEBUG_PRINT {
            println!("Inflight queries: {:?}", self.inflight_queries.iter().map(|(_, q)| (q.source, q.var, q.updates_left)).collect::<Vec<_>>());
            println!("Query buffer: {:?}", self.query_buffer.iter().map(|q| (q.source, q.var)).collect::<Vec<_>>());
        }
        // remove queries that have been processed (0 updates left)
        self.inflight_queries.retain(|_, query| query.updates_left > 0);

        // Iterate through the inflight queries, send appropriate messages to network
        for (_, query) in self.inflight_queries.iter_mut() {
            
            let clause_to_check = self.num_clauses - query.updates_left; // Range = [0, num_clauses - 1]

            let mut returning_message = [TermUpdate::Unchanged; CLAUSE_LENGTH]; // Initialize the bitmask to 0

            // iterate through the clause_to_check
            for i in 0..CLAUSE_LENGTH {
                let clause = self.clause_table[clause_to_check][i]; // Get the clause from the clause table

                if query.var == clause.var {
                    if query.set == !clause.negated {
                        returning_message[i] = TermUpdate::True;
                    } else {
                        returning_message[i] = TermUpdate::False;
                    }             
                } else if query.reset && clause.var > query.var {
                    returning_message[i] = TermUpdate::Reset;
                }
            }

            // Send a message with returningBitmask to the network
            query.updates_left -= 1; // Decrement the number of updates left to process

            let from = MessageDestination::ClauseTable;

            let to = MessageDestination::Neighbor(query.source);


            let message = Message::SubstitutionMask {
                mask: returning_message,
            };

            network.start_message(from, to, message);
            
        }
        
    }

    fn send_message(&self, network: &mut MessageQueue, to: MessageDestination, message: Message) {
        network.start_message(MessageDestination::ClauseTable, to, message);
    }

    pub fn recieve_message(&mut self, from: MessageDestination, message: Message) {
        match (from, message) {
            (MessageDestination::Neighbor(node_id), Message::SubsitutionQuery { id, assignment, reset }) => {
                self.query_buffer.push_back(
                    Query {
                        source: node_id,
                        var: id,
                        set: assignment,
                        reset: reset,
                        updates_left: self.num_clauses,
                    }
                );
            }
            (MessageDestination::Neighbor(node_id), Message::SubstitutionAbort) => {
                Query {
                    source: node_id,
                    var: 0,
                    set: false,
                    reset: false,
                    updates_left: 0,  // updates_left = 0 means that the query will replace and immediately be removed
                };
            }
            _ => {
                panic!("Invalid message type for ClauseTable");
            }
        }
    }

    pub fn get_blank_state(&self) -> CNFState {
        return vec![[TermState::Symbolic; 3]; self.num_clauses];
        // 0s in all terms for each clause
    }

    pub fn clone_table(&self) -> Vec<[Term; CLAUSE_LENGTH]> {
        self.clause_table.clone()
    }
}