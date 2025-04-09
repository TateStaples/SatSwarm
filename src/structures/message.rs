use std::fmt::Debug;

use crate::{DEBUG_PRINT};

use super::{clause_table::ClauseTable, node::{NodeId, SpeculativeDepth, VarId}};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageDestination {
    Neighbor(NodeId),
    Broadcast, 
} 
#[derive(Clone)]
pub enum Message {
    Fork {
        table: ClauseTable,  // CNF assignment buffer state
        assigned_vars: Vec<SpeculativeDepth>,   // List of already assigned variables (later work can make this more complex)
    },
    Success,
} impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Fork {..} => {
                write!(f, "Fork")
            },
            Message::Success => {
                write!(f, "Success")
            },
        }
    }
}

pub struct Watchdog {
    last_update: u64,
    clock: &'static u64,
    timeout: u64,
} impl Watchdog {
    pub fn new(clock: u64, timeout: u64) -> Self {
        Watchdog {
            last_update: *clock,
            clock,
            timeout
        }
    }

    fn reset(&mut self) {
        self.last_update = *self.clock;
    }

    pub fn peek(&self) -> bool {
        let result = *self.clock - self.last_update > self.timeout;
        assert!(!result, "Watchdog timeout: last update: {}, current time: {}, timeout: {}", self.last_update, *self.clock, self.timeout);
        return *self.clock - self.last_update > self.timeout;
    }

    pub fn check(&mut self) -> bool {
        let result = if self.peek() {
            true
        } else {
            false
        };
        self.reset();
        return result;
    }
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
        let arrival = (self.head + delay) % N;
        self.buffer[arrival].push(item);
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
    bandwidth: usize,
    queue: CircularBuffer<(MessageDestination, MessageDestination, Message), 64>
}
impl MessageQueue {
    pub fn new() -> Self {
        MessageQueue {
            clock: get_clock(),
            last_clock_update: *get_clock(),
            queue: CircularBuffer::new(),
            bandwidth: 1_000,
        }
    }

    pub fn set_bandwidth(&mut self, bandwidth: usize) {
        self.bandwidth = bandwidth;
    }

    fn check_clock(&mut self) {
        for _ in self.last_clock_update..*self.clock {
            self.queue.step();
        }
        self.last_clock_update = *self.clock;
    }

    pub fn start_message(&mut self, from: MessageDestination, to: MessageDestination, message: Message) {
        self.check_clock();
        if DEBUG_PRINT {
            println!("Sending {:?} from {:?} to {:?}", message, from, to);
        }
        let delay = match message {
            Message::Fork {..} => (std::mem::size_of::<CNFState>() + std::mem::size_of::<VarId>() - 1) / self.bandwidth + 1,
            _ => 1,
        };
        for i in 1..delay {
            self.queue.push(i, (from, to, Message::UnfinishedMessage)); 
        }
        self.queue.push(delay, (from, to, message));  // TODO: add more realistic delays
    }

    pub fn pop_message(&mut self) -> Vec<(MessageDestination, MessageDestination, Message)> {
        self.check_clock();
        let result = self.queue.pop();
        if DEBUG_PRINT {
            println!("Popping {:?}", result);
        }
        return result;
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TermUpdate {
    Unchanged,
    True,
    False,
    Reset
}
