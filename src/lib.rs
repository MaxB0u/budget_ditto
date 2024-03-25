use pnet::datalink;
use pnet::datalink::Channel::Ethernet;
use pnet::packet::ethernet::EthernetPacket;
use pnet::util::MacAddr;
//use std::collections::BinaryHeap;
use std::error::Error;
use std::sync::{Arc, Mutex};
mod queues;
pub use crate::queues::priority_queue;
pub use crate::queues::round_robin;

mod pattern;

struct ChannelCustom {
    interface: datalink::NetworkInterface,
    tx: Box<dyn datalink::DataLinkSender>,
    rx: Box<dyn datalink::DataLinkReceiver>,
}

pub fn run(rx_interface: &str, tx_interface: &str) -> Result<(), Box<dyn Error>> {
    // Test priority queues
    priority_queue::try_priority_queue();
    
    let mut ch_rx = match get_channel(rx_interface) {
        Ok(rx) => rx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    let mut ch_tx = match get_channel(tx_interface) {
        Ok(tx) => tx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    println!("Setting up queues for pattern {:?}", pattern::PATTERN);
    let rrs = Arc::new(Mutex::new(round_robin::RoundRobinScheduler::new(pattern::PATTERN[0])));

    println!("Listening for Ethernet frames on interface {}...", rx_interface);
    println!("Sending Ethernet frames on interface {}...", tx_interface);

    // Should start a thread for rx and one for send
    try_receive(&mut ch_rx.rx);

    transmit(&mut ch_tx.tx, &rrs);
    receive(&mut ch_rx.rx, &rrs);

    Ok(())
}

fn get_channel(interface_name: &str) -> Result<ChannelCustom, &'static str>{
    // Retrieve the network interface
    let interfaces = datalink::interfaces();
    let interface = match interfaces
        .into_iter()
        .find(|iface| iface.name == interface_name) {
            Some(inter) => inter,
            None => return Err("Failed to find network interface"),
        };

    // Create a channel to receive Ethernet frames
    let (tx, rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => return Err("Unknown channel type"),
        Err(e) => panic!("Failed to create channel {e}"),
    };

    let ch = ChannelCustom{
        interface, 
        tx, 
        rx,
    };

    Ok(ch)
}

fn transmit(tx: &mut Box<dyn datalink::DataLinkSender>, rrs: &Arc<Mutex<round_robin::RoundRobinScheduler>>) {

    // Send Ethernet frames
    loop {
        let mut scheduler = rrs.lock().unwrap();
        match tx.send_to(scheduler.pop(), None) {
            Some(res) => {
                
                match res {
                    Ok(_) => (),
                    Err(e) => eprintln!("Error sending frame: {}", e),
                }
            }
            None => {
                eprintln!("No packets to send");
            }
        }
    }
}

fn receive<'a, 'b>(rx: &'a mut Box<dyn datalink::DataLinkReceiver>, rrs: &'b Arc<Mutex<round_robin::RoundRobinScheduler<'b>>>) 
where 'a: 'b {
    // Process received Ethernet frames
    loop {
        match rx.next() {
            Ok(packet) => process_packet(packet, rrs),
            Err(e) => {
                eprintln!("Error receiving frame: {}", e);
                continue;
            }
        };
    }
}

fn process_packet<'a, 'b>(packet: &'a [u8], rrs: &'b Arc<Mutex<round_robin::RoundRobinScheduler<'b>>>) 
where 'a: 'b {
    let eth_packet = EthernetPacket::new(packet).unwrap();
    if eth_packet.get_source() == MacAddr::new(2,0,0,0,0,0){
        let mut scheduler = rrs.lock().unwrap();
        scheduler.push(packet);
    }
}


fn try_receive(rx: &mut Box<dyn datalink::DataLinkReceiver>) {
    // Arrays of pattern length used to send data
    let mut padded_pkt: [u8; pattern::PATTERN[0]] = [0; pattern::PATTERN[0]];

    // Process received Ethernet frames
    loop {
        match rx.next() {
            Ok(packet) => {
                
                let eth_packet = EthernetPacket::new(packet).unwrap();
                if eth_packet.get_source() == MacAddr::new(2,0,0,0,0,0){
                    println!("Received Ethernet frame: {:?}", eth_packet);
                    println!("The raw packet was {:?} of length {}", packet, packet.len());

                    padded_pkt[..packet.len()].copy_from_slice(packet);

                    println!("The padded packet is {:?} with length {}", padded_pkt, padded_pkt.len());
                }
            }
            Err(e) => {
                eprintln!("Error receiving frame: {}", e);
            }
        }
    }
}