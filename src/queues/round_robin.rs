use crate::queues::priority_queue;
use crate::pattern;

pub struct RoundRobinScheduler {
    pub queues: Vec<priority_queue::PriorityQueue>,
    pub current_q: usize,
    pub sorted_indices: Vec<usize>,
}

impl RoundRobinScheduler {
    pub fn new(num_queues: usize) -> RoundRobinScheduler {
        let mut queues = Vec::with_capacity(num_queues);
        for i in 0..num_queues {
            queues.push(priority_queue::PriorityQueue::new(pattern::PATTERN[i]));
        }
        RoundRobinScheduler {
            queues,
            current_q: 0,
            sorted_indices: pattern::get_sorted_indices(),
        }
    }

    pub fn push(&mut self, packet: Vec<u8>) {
        for i in 0..self.queues.len() {
            // Look if fits in pattern from smallest to largest element
            if packet.len() <= pattern::PATTERN[self.sorted_indices[i]] { // Look into making this more efficient
                self.queues[i].push(packet);
                break;
            }
        }
    }

    pub fn pop(&mut self) -> Vec<u8> {
        let queue_index = self.current_q;
        self.current_q = (self.current_q + 1) % self.queues.len();
        // Pop from the current queue
        self.queues[queue_index].pop()
    }
}