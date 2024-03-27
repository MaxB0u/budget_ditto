
use std::collections::BinaryHeap;
use std::cmp::Reverse;
use std::cmp::Ordering;
// use pnet::packet::ipv4::MutableIpv4Packet;
// use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::Packet;
use std::collections::VecDeque;
use crate::pattern;
use pnet::packet::ethernet;
use pnet::util::MacAddr;

// Higher priority served first
const PRIORITY_CHAFF: i32 = 1;
const PRIORITY_REAL: i32 = 2;

// const IP_HEADER_LENGTH: usize = 20;
// const IP_VERSION: u8 = 4;
// const IP_PROTOCOL_IP_IN_IP: u8 = 4;

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

pub struct PriorityQueue {
    // Might be more efficient to hard code a queue length in an array
    pub queue: VecDeque<Vec<u8>>,
    pub length: usize,
}

impl PriorityQueue {
    pub fn new(length: usize) -> Self{
        PriorityQueue{queue: VecDeque::new(), length}
    }

    pub fn push(&mut self, packet: Vec<u8>) {
        // Pad when you push to be more efficient when you pop
        let padded_data = pad(packet, self.length);
        self.queue.push_back(padded_data);
        //println!("Queue length {}", self.queue.len());
    }

    pub fn pop(&mut self) -> Vec<u8> {
       let packet = match self.queue.pop_front() {
           Some(pkt) => {
            println!("Transmit real packet length {} with first byte after addresses {}", pkt.len(), pkt[12]);
            assert_ne!(pkt[12], 0_u8); // Make sure first byte after addresses is not 0 or else will be seen as chaff
            pkt
           },
           None => {
            pattern::CHAFF[..self.length].to_vec()
           }
       };
       //pad_and_wrap_ipv4(packet, self.length)
       packet
    }
}

fn pad(data: Vec<u8>, target_length: usize) -> Vec<u8> {
    // Unefficient, copies data to a vector
    let initial_len = data.len();
    let mut padded_data = data;

    let mut eth_packet = ethernet::MutableEthernetPacket::new(&mut padded_data).unwrap();
    // Encode length without padding in src mac address (in little endian)
    eth_packet.set_source(MacAddr::new(0_u8,0_u8,0_u8,0_u8, (initial_len & 0xFF) as u8, ((initial_len >> 8) & 0xFF) as u8));
    padded_data = eth_packet.packet().to_vec();
    
    padded_data.resize(target_length, 0);

    //println!("Padded length {}", padded_data.len());
    padded_data
}


// fn pad_and_wrap_ipv4(data: Vec<u8>, target_length: usize) -> Vec<u8> {
//     let initial_len = data.len();
//     let mut data = data;

//     let mut eth_packet = ethernet::MutableEthernetPacket::new(&mut data).unwrap();
//     // Encode length without padding in src mac address
//     eth_packet.set_source(MacAddr::new(0_u8,0_u8,0_u8,0_u8, ((initial_len >> 8) & 0xFF) as u8, (initial_len & 0xFF) as u8));
//     data = eth_packet.packet().to_vec();
    
//     data.resize(target_length + IP_HEADER_LENGTH, 0);
//     data.rotate_right(IP_HEADER_LENGTH);
//     let mut packet = MutableIpv4Packet::new(&mut data).unwrap();

//     // Set the IP header fields
//     packet.set_version(IP_VERSION);
//     packet.set_header_length((IP_HEADER_LENGTH/4) as u8);
//     packet.set_total_length(((initial_len + IP_HEADER_LENGTH)) as u16); // Set the total length of the packet
//     //packet.set_identification(1234);
//     packet.set_ttl(64);
//     packet.set_next_level_protocol(IpNextHeaderProtocols::Tcp); // Assuming TCP protocol
//     packet.set_source([192, 168, 1, 1].into());
//     packet.set_destination([192, 168, 1, 2].into());

//     // Calculate the checksum for the IP header
//     packet.set_checksum(pnet::packet::ipv4::checksum(&packet.to_immutable()));
    
//     packet.packet().to_vec()
// }

