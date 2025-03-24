use std::{collections::VecDeque, io::BufRead, path::PathBuf}; // Import VecDeque for FIFO queue

use crate::{get_clock, GLOBAL_CLOCK};

use super::node::{CNFState, NodeId, TermState, VarId, CLAUSE_LENGTH}; // Import Network struct
use super::message::{Message, MessageDestination, MessageQueue, TermUpdate}; // Import Message struct
struct Query {
    source: NodeId,
    var: VarId,
    set: bool,
    reset: bool,
    updates_left: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Default)]
struct Term {
    var: VarId,
    negated: bool,
}
pub struct ClauseTable {
    clause_table: Vec<[Term; CLAUSE_LENGTH]>, // 2D Vec to store the table of clauses

    num_clauses: usize,           // Number of clauses in the table
    clock: &'static u64,      // Reference to the global clock

    query_buffer: VecDeque<Query>, // FIFO queue to hold incoming queries
    // Hey, Shaan just added in queries into the struct because it seemed simpler than having a separate hashmap
    inflight_queries: Vec<Query>, // hashmap query: VarId -> updates_left_to_process: u64
}

impl ClauseTable {
    // Creates a new ClauseTable
    pub fn dummy() -> Self {
        let num_clauses = 10; // Number of clauses in the table
        Self {
            clause_table: vec![[Default::default(); CLAUSE_LENGTH]; num_clauses as usize], // Initialize the clause table with 0s
            num_clauses: num_clauses, // Initialize the number of clauses
            clock: get_clock(), // Initialize the clock reference
            query_buffer: VecDeque::new(), // Initialize an empty FIFO queue
            inflight_queries: Vec::new(), // Initialize an empty hashmap
        }
    }

    pub fn load_file(file: PathBuf) -> Self{
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
            } else {
                let parts = line.split_whitespace();
                for (term_index, part) in parts.enumerate() {
                    assert!(!clause_end, "Clause has already ended");
                    let num: i32 = part.parse().unwrap();
                    if num == 0 {
                        clause_end = true;
                        assert!(term_index == CLAUSE_LENGTH, "Only 3SAT is supported");
                    } else {
                        assert!(num.abs()-1 < u8::MAX as i32, "Too many variables for u8");
                        clause[term_index] = Term{var: (num.abs()-1) as u8, negated: num < 0};  // want to 0 index the variables
                    }
                }
            }
            if clause_end {
                clauses.push(clause);
                clause = [Term{var: 0, negated: false}; CLAUSE_LENGTH];
            }
        }
        assert!(clauses.len() == num_clauses, "Number of clauses does not match header");
        assert!(clauses.iter().map(|c| c.iter().map(|t| t.var).max().unwrap()).max().unwrap() < var_count as u8, "Variable count does not match header");
        Self {
            clause_table: clauses,
            num_clauses: num_clauses,
            clock: get_clock(),
            query_buffer: VecDeque::new(),
            inflight_queries: Vec::new(),
        }
    }
    // Updates the ClauseTable
    pub fn clock_update(&mut self, network: &mut MessageQueue) {
        
        // try to see if any new queries have been added to the query buffer
        if !self.query_buffer.is_empty() {
            let query = self.query_buffer.pop_front().unwrap(); // Get the first query from the queue
            self.inflight_queries.push(query); // Add the query to the inflight queries
        }

        // Iterate through the inflight queries, send appropriate messages to network
        for query in self.inflight_queries.iter_mut() {
            
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

        // remove queries that have been processed (0 updates left)
        self.inflight_queries.retain(|query| query.updates_left > 0);
        
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
            _ => {
                panic!("Invalid message type for ClauseTable");
            }
        }
    }

    pub fn get_blank_state(&self) -> CNFState {
        return vec![[TermState::Symbolic; 3]; self.num_clauses];
        // 0s in all terms for each clause
    }
}