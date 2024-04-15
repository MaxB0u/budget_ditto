use pnet::packet::ipv4;
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::Packet;
use crossbeam::queue::ArrayQueue;
use crate::pattern;
// use pnet::packet::ethernet;
// use pnet::util::MacAddr;

const IP_HEADER_LENGTH: usize = 20;
const IP_VERSION: u8 = 4;
// const IP_PROTOCOL_IP_IN_IP: u8 = 4;
const MAX_Q_LEN: usize = 1024;

const SRC_IP_ADDR: [u8;4] = [10, 9, 0, 2];
const DST_IP_ADDR: [u8;4] = [10, 9, 0, 1];

pub struct PriorityQueue {
    // Might be more efficient to hard code a queue length in an array
    pub queue: ArrayQueue<Vec<u8>>,
    pub length: usize,
}

impl PriorityQueue {
    pub fn new(length: usize) -> Self{
        PriorityQueue{queue: ArrayQueue::new(MAX_Q_LEN), length}
    }

    pub fn push(&self, packet: Vec<u8>) {
        // Pad when you push to be more efficient when you pop
        let padded_data = pad(packet, self.length);
        if let Err(_) = self.queue.push(padded_data) {
            println!("Queue {} full, length = {}, error pushing", self.length, self.queue.len());
        }
        //println!("Queue length {}", self.queue.len());
    }

    // pub fn push_no_reorder(&self, packet: Vec<u8>, is_chaff: bool) {
    //     // Pad when you push to be more efficient when you pop
    //     if is_chaff {
    //         if let Err(_) = self.queue.push(packet) {
    //             println!("Queue {} full, length = {}, error pushing", self.length, self.queue.len());
    //         }
    //     } else {
    //         let padded_data = pad(packet, self.length);
    //         if let Err(_) = self.queue.push(padded_data) {
    //             println!("Queue {} full, length = {}, error pushing", self.length, self.queue.len());
    //         }
    //     }
    // }

    pub fn pop(&self) -> Vec<u8> {
       let packet = match self.queue.pop() {
           Some(pkt) => {
            //println!("Transmit real packet length {} with first byte after addresses {}", pkt.len(), pkt[12]);
            //assert_ne!(pkt[12], 0_u8); // Make sure first byte after addresses is not 0 or else will be seen as chaff
            pkt
           },
           None => {
            pattern::CHAFF[..self.length].to_vec()
            //rand::thread_rng().sample_iter(self.distribution).take(self.length).collect()
           }
       };
       //packet
       wrap_in_ipv4(packet)
    }
}

fn pad(data: Vec<u8>, target_length: usize) -> Vec<u8> {
    // Unefficient, copies data to a vector
    // let initial_len = data.len();
    // let mut padded_data = data;

    // let mut eth_packet = ethernet::MutableEthernetPacket::new(&mut padded_data).unwrap();
    // // Encode length without padding in src mac address (in little endian)
    // eth_packet.set_source(MacAddr::new(0_u8,0_u8,0_u8,0_u8, (initial_len & 0xFF) as u8, ((initial_len >> 8) & 0xFF) as u8));
    // padded_data = eth_packet.packet().to_vec();
    
    // padded_data.resize(target_length, 0);

    // //println!("Padded {}B", padded_data.len() - initial_len);
    // padded_data

    let mut padded_data = data;
    // let mut packet = ipv4::MutableIpv4Packet::new(&mut padded_data).unwrap();
    // padded_data = packet.packet().to_vec();
    padded_data.resize(target_length, 0);
    //println!("Padded {}B", padded_data.len() - initial_len);
    padded_data
}


fn wrap_in_ipv4(data: Vec<u8>) -> Vec<u8> {
    let initial_len = data.len();
    let mut data = data;
    
    data.resize(initial_len + IP_HEADER_LENGTH, 0);
    data.rotate_right(IP_HEADER_LENGTH);
    let mut packet = ipv4::MutableIpv4Packet::new(&mut data).unwrap();

    // Set the IP header fields
    packet.set_version(IP_VERSION);
    packet.set_header_length((IP_HEADER_LENGTH/4) as u8);
    packet.set_total_length(((initial_len + IP_HEADER_LENGTH)) as u16); // Set the total length of the packet
    //packet.set_identification(1234);
    packet.set_ttl(64);
    packet.set_next_level_protocol(IpNextHeaderProtocols::IpIp); 
    packet.set_source(SRC_IP_ADDR.into());
    packet.set_destination(DST_IP_ADDR.into());

    packet.set_checksum(pnet::packet::ipv4::checksum(&packet.to_immutable()));
    
    packet.packet().to_vec()
}

