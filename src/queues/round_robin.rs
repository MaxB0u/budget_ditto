use std::sync::Mutex;
use crate::queues::priority_queue;
use crate::pattern;

pub static TOTAL_PAD: Mutex<f64> = Mutex::new(0.0);

pub struct RoundRobinScheduler {
    pub queues: Vec<priority_queue::PriorityQueue>,
    pub current_q: usize,
    pub sorted_indices: Vec<usize>,
    pub pps: f64
}

impl RoundRobinScheduler {
    pub fn new(num_queues: usize, pps: f64) -> RoundRobinScheduler {
        let mut queues = Vec::with_capacity(num_queues);
        for i in 0..num_queues {
            queues.push(priority_queue::PriorityQueue::new(pattern::PATTERN[i]));
        }
        RoundRobinScheduler {
            queues,
            current_q: 0,
            sorted_indices: pattern::get_sorted_indices(),
            pps: pps,
        }
    }

    pub fn push(&self, packet: Vec<u8>) {
        let mut is_pushed = false;
        let length = packet.len();
        for i in 0..self.queues.len() {
            // Look if fits in pattern from smallest to largest element
            if packet.len() <= pattern::PATTERN[self.sorted_indices[i]] { // Look into making this more efficient
                self.queues[i].push(packet);
                is_pushed = true;
                
                // Keep track of total padding]
                let mut data = TOTAL_PAD.lock().unwrap();
                *data += (self.queues[i].length - length) as f64 / self.pps;
                break;
            }
        }
        if !is_pushed {
            //println!("Could not push packet of length {}", length);
        }
    }

    pub fn pop(&self, idx: usize) -> Vec<u8> {
        // Pop from the current queue
        self.queues[idx].pop()
    }
}