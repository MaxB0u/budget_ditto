
use std::collections::BinaryHeap;
use std::cmp::Reverse;
use std::cmp::Ordering;

use crate::pattern;

// Higher priority served first
const PRIORITY_CHAFF: i32 = 1;
const PRIORITY_REAL: i32 = 2;

// Each packet has a priority and a reference to its data
#[derive(Debug)]
pub struct PriorityPacket<'a> {
    pub priority: i32,
    pub data: &'a [u8], 
}

// PartialEq to compare based on priorities
impl<'a> PartialEq for PriorityPacket<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

// The Eq trait has a default implementation
impl<'a> Eq for PriorityPacket<'a> {}

// PartialOrd to compare priorities
impl<'a> PartialOrd for PriorityPacket<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// Ord to compare priorities
impl<'a> Ord for PriorityPacket<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority) // Reverse ordering for max-heap
    }
}

pub fn try_priority_queue() {
    // Create an empty binary heap priority queue
    let mut priority_queue: BinaryHeap<Reverse<PriorityPacket>> = BinaryHeap::new();

    let priority_pkt_5 = PriorityPacket{priority: PRIORITY_CHAFF, data: &[0; 1]};
    let priority_pkt_2 = PriorityPacket{priority: PRIORITY_REAL, data: &[1; 1]};
    let priority_pkt_7 = PriorityPacket{priority: PRIORITY_CHAFF, data: &[2; 1]};

    // Insert packets into the priority queues 
    priority_queue.push(Reverse(priority_pkt_5)); 
    priority_queue.push(Reverse(priority_pkt_2)); 
    priority_queue.push(Reverse(priority_pkt_7)); 

    // Pop elements from the priority queue in order of priority
    while let Some(Reverse(priority_pkt)) = priority_queue.pop() {
        println!("Popped element: {:?}", priority_pkt);
    }
}

pub struct PriorityQueue<'a> {
    // Might be more efficient to hard code a queue length in an array
    pub queue: Vec<&'a[u8]>,
    pub length: usize,
}

impl<'a> PriorityQueue<'a> {
    pub fn new(length: usize) -> Self{
        PriorityQueue{queue: Vec::new(), length}
    }

    pub fn push(&mut self, packet: &'a[u8]) {
        // Pad when you push to be more efficient when you pop
        self.queue.push(pad(packet, self.length),);
    }

    pub fn pop(&mut self) -> &'a [u8] {
       let packet = match self.queue.pop() {
           Some(pkt) => pkt,
           None => &pattern::CHAFF[0..self.length],
       };
       packet
    }
}

pub fn pad(data: &[u8], target_length: usize) -> &[u8] {
    // // Calculate the number of zeros needed for padding
    // let num_zeros = target_length - data.len();

    // let data_len = data.len();
    // let data_ptr = data.as_ptr() as *mut i32;
    // unsafe {
    //     std::ptr::write_bytes(data_ptr.add(data_len), 0, num_zeros);
    // }

    // data

    // Unefficient, copies data to a vector
    data.to_vec().resize(target_length , 0);
    data
}


