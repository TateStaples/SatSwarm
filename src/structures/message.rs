use std::fmt::Debug;

use super::{clause_table::{CNFState, ClauseTable}, util_types::{NodeId, VarId, DEBUG_PRINT}};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageDestination {
    Neighbor(NodeId),
    Broadcast, 
} 

pub enum Message {
    Fork {
        assignments: Vec<(VarId, bool, bool)>  // The hardware implementation should be a bit different
    },
    UnfinishedMessage,
    Success,
} impl Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Fork {..} => {
                write!(f, "Fork")
            },
            Message::UnfinishedMessage => {
                write!(f, "UnfinishedMessage")
            },
            Message::Success => {
                write!(f, "Success")
            },
        }
    }
}

pub struct Watchdog {
    last_update: u64,
    timeout: u64,
} impl Watchdog {
    pub fn new(clock: u64, timeout: u64) -> Self {
        Watchdog {
            last_update: clock,
            timeout
        }
    }

    fn reset(&mut self, clock: u64) {
        self.last_update = clock;
    }

    pub fn peek(&self, clock: u64) -> bool {
        let result = clock - self.last_update > self.timeout;
        assert!(!result, "Watchdog timeout: last update: {}, current time: {}, timeout: {}", self.last_update, clock, self.timeout);
        return clock - self.last_update > self.timeout;
    }

    pub fn check(&mut self, clock: u64) -> bool {
        let result = if self.peek(clock) {
            true
        } else {
            false
        };
        self.reset(clock);
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
    bandwidth: usize,
    queue: CircularBuffer<(MessageDestination, MessageDestination, Message), 64>
}
impl MessageQueue {
    pub fn new() -> Self {
        MessageQueue {
            last_clock_update: 0,
            queue: CircularBuffer::new(),
            bandwidth: 1_000,
        }
    }

    pub fn set_bandwidth(&mut self, bandwidth: usize) {
        self.bandwidth = bandwidth;
    }

    fn check_clock(&mut self, clock: u64) {
        for _ in self.last_clock_update..clock {
            self.queue.step();
        }
        self.last_clock_update = clock;
    }

    pub fn start_message(&mut self, clock: u64, from: MessageDestination, to: MessageDestination, message: Message) {
        self.check_clock(clock);
        if DEBUG_PRINT {
            println!("Sending {:?} from {:?} to {:?}", message, from, to);
        }
        let delay = match message {
            // Message::Fork {..} => (std::mem::size_of::<CNFState>() + std::mem::size_of::<VarId>() - 1) / self.bandwidth + 1,
            // TODO: if we think that the size of the message is less than can be processed in a clock cycle we can just set the delay to 1
            _ => 1,
        };
        for i in 1..delay {
            self.queue.push(i, (from, to, Message::UnfinishedMessage)); 
        }
        self.queue.push(delay, (from, to, message));  // TODO: add more realistic delays
    }

    pub fn pop_message(&mut self, clock: u64) -> Vec<(MessageDestination, MessageDestination, Message)> {
        self.check_clock(clock);
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
