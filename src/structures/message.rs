use std::fmt::Debug;

use crate::get_clock;

use super::node::{CNFState, NodeId, VarId, CLAUSE_LENGTH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    last_clock_update: u64,
    clock: &'static u64,
    queue: CircularBuffer<(MessageDestination, MessageDestination, Message), 64>
}
impl MessageQueue {
    pub fn new() -> Self {
        MessageQueue {
            clock: get_clock(),
            last_clock_update: 0,
            queue: CircularBuffer::new()
        }
    }

    fn check_clock(&mut self) {
        for _ in self.last_clock_update..*self.clock {
            self.queue.step();
        }
        self.last_clock_update = *self.clock;
    }

    pub fn start_message(&mut self, from: MessageDestination, to: MessageDestination, message: Message) {
        self.check_clock();
        self.queue.push(1, (from, to, message));  // TODO: add more realistic delays
    }

    pub fn pop_message(&mut self) -> Vec<(MessageDestination, MessageDestination, Message)> {
        self.check_clock();
        self.queue.pop()
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermUpdate {
    Unchanged,
    True,
    False,
    Reset
}
#[derive(Clone, PartialEq, Eq, Hash)]
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
} impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Fork {cnf_state, assigned_vars} => {
                write!(f, "Fork")
            },
            Message::Success => {
                write!(f, "Success")
            },
            Message::SubstitutionMask {mask} => {
                write!(f, "SubstitutionMask {{mask: {:?}}}", mask)
            },
            Message::SubsitutionQuery {id, assignment, reset} => {
                write!(f, "SubsitutionQuery {{id: {}, assignment: {}, reset: {}}}", id, assignment, reset)
            }
        }
    }
    
}