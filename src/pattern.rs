// use crate::priority_queue::PriorityQueue;
// use std::collections::BinaryHeap;
// use std::rc::Rc;
// use std::cell::RefCell;

pub const PATTERN: [usize; 3] = [500, 1000, 1400]; // 86B overhead with VPN: 1428+86=1514B -> Or else fragment
//pub const PATTERN: [usize; 2] = [782, 1500];
//pub const PATTERN: [usize; 1] = [1500];
// Largest size possible in pattern
const MTU: usize = 1500;
pub const CHAFF: [u8; MTU] = [0; MTU];

// pub fn get_chaff_pkts<'a>() -> Vec<&'a [u8]> {
//     let mut chaff_pkts = Vec::<&[u8]>::new();
//     for &length in PATTERN.iter() {
//         // Since can not use constant values directly, need this workaround
//         let chaff: &'a [u8] = &[0_u8; MTU];
//         chaff_pkts.push(&chaff[0..length]);
//     }
//     chaff_pkts
// }

pub fn get_sorted_indices() -> Vec<usize> {
    // Gets sorted indices needed to match incoming packets and the corresponding queue index to choose
    let mut indices: Vec<usize> = (0..PATTERN.len()).collect();
    // Sort the indices based on the corresponding values in the data vector
    indices.sort_by_key(|&i| &PATTERN[i]);
    indices
}

// pub fn get_priority_queues<'a>() -> Vec<PriorityQueue<'a>> {
//     let mut priority_queues = Vec::<PriorityQueue>::new();

//     // Create BinaryHeaps and wrap them in Rc<RefCell<_>>
//     let binary_heaps: Vec<_> = PATTERN.iter().map(|_| Rc::new(RefCell::new(BinaryHeap::new()))).collect();

//     // Iterate over PATTERN and create PriorityQueue instances
//     for binary_heap_rc in &binary_heaps {
//         let pq = PriorityQueue { queue: Rc::clone(&binary_heap_rc) };
//         priority_queues.push(pq);
//     }

//     priority_queues
// }