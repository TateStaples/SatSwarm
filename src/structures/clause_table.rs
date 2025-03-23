use std::collections::VecDeque; // Import VecDeque for FIFO queue

use super::node::{CNFState, Message, MessageDestination, MessageQueue, CLAUSE_LENGTH}; // Import Network struct
use std::collections::HashMap; // Import HashMap for query mapping

pub struct ClauseTable {
    clause_table: Vec<[u8; CLAUSE_LENGTH]>, // 2D Vec to store the table of clauses

    num_clauses: usize,           // Number of clauses in the table
    clock: &'static u64,      // Reference to the global clock

    query_buffer: VecDeque<u8>, // FIFO queue to hold incoming queries

    // HashMap to keep track of inflight queries, and the number of updates left to process
    inflight_queries: HashMap<u8, usize>, // hashmap query: u8 -> updates_left_to_process: u64
}

impl ClauseTable {
    // Creates a new ClauseTable
    pub fn new(num_clauses: usize, clock: &'static u64) -> Self {
        Self {
            clause_table: vec![[0; CLAUSE_LENGTH]; num_clauses as usize], // Initialize the clause table with 0s
            num_clauses: num_clauses, // Initialize the number of clauses
            clock,
            query_buffer: VecDeque::new(), // Initialize an empty FIFO queue
            inflight_queries: HashMap::new(), // Initialize an empty hashmap
        }
    }

    // Updates the ClauseTable
    pub fn clock_update(&mut self, network: &mut MessageQueue) {
        
        // try to see if any new queries have been added to the query buffer
        if !self.query_buffer.is_empty() {
            let query = self.query_buffer.pop_front().unwrap(); // Get the first query from the queue
            self.inflight_queries.insert(query, self.num_clauses); // Add the query to the inflight queries hashmap
        }

        // Iterate through the inflight queries, send appropriate messages to network
        for (query, updates_left) in self.inflight_queries.iter_mut() {
            
            let clause_to_check = self.num_clauses - *updates_left; // Range = [0, num_clauses - 1]

            let mut returningMessage = [0; CLAUSE_LENGTH]; // Initialize the bitmask to 0

            // iterate through the clause_to_check
            for i in 0..CLAUSE_LENGTH {
                let clause = self.clause_table[clause_to_check][i]; // Get the clause from the clause table

                if *query == clause {
                    // TODO: need to figure out negation
                    returningMessage[i] = 0b10; // positive
                }
            }

            // Send a message with returningBitmask to the network
            *updates_left -= 1; // Decrement the number of updates left to process

            let from = MessageDestination::ClauseTable;

            let to = MessageDestination::Neighbor(NodeId); 


            let message = Message::SubstitutionMask {
                mask: returningMessage,
            };

            network.start_message(from, to, message);
            
        }

        // remove queries that have been processed (0 updates left)
        self.inflight_queries.retain(|_, updates_left| *updates_left > 0);
        
    }

    pub fn recieve_message(&mut self, from: MessageDestination, message: Message) {
        todo!();
    }

    
    // Adds a variable to the query buffer

    // TODO: update query
    pub fn query(&mut self, var: u8) {

        // TODO: Check for occupancy of this buffer - too much is bad

        self.query_buffer.push_back(var); // Add the variable to the end of the queue
    }

    pub fn get_blank_state(&self) -> CNFState {
        todo!();
        // 0s in all terms for each clause
    }
}