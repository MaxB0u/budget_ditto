mod pattern;
mod deobfuscate;
mod queues;

use crate::queues::round_robin;
use pnet::datalink;
use pnet::datalink::Channel::Ethernet;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// Rate is Packet/s * Bytes/packet
const PACKETS_PER_SECOND: f64 = 1e4;//1e1; // -> 100micros between packets

pub struct ChannelCustom {
    pub tx: Box<dyn datalink::DataLinkSender>,
    pub rx: Box<dyn datalink::DataLinkReceiver>,
}

pub struct Interfaces {
    pub input: String,
    pub obfuscated_output: String,
    pub obfuscated_input: String,
    pub output: String,
}

pub fn run(interfaces: Interfaces) -> Result<(), Box<dyn Error>> {
    
    // let devices = Device::list()?;
    // for device in &devices {
    //     println!("Device: {}", device.name);
    // }

    println!("Setting up queues for pattern {:?}", pattern::PATTERN);
    let rrs = Arc::new(Mutex::new(round_robin::RoundRobinScheduler::new(pattern::PATTERN.len())));

    let tx_queue = Arc::clone(&rrs);
    let rx_queue = Arc::clone(&rrs);

    println!("Listening for Ethernet frames on interface {}...", interfaces.input);
    println!("Sending obfuscated Ethernet frames on interface {}...", interfaces.obfuscated_output);
    println!("Listening for obfuscated Ethernet frames on interface {}...", interfaces.obfuscated_input);
    println!("Sending deobfuscated Ethernet frames on interface {}...", interfaces.output);

    // Spawn thread for obfuscating packets
    let obf_handle = thread::spawn(move || {
        obfuscate(&interfaces.input, rx_queue);
    });

    // Spawn thread for sending obfuscated packets
    let send_handle = thread::spawn(move || {
        transmit(&interfaces.obfuscated_output, tx_queue);
    });

    // Spawn thread for sending deobfuscating and forwarding packets
    let deobf_handle = thread::spawn(move || {
        deobfuscate(&interfaces.obfuscated_input, &interfaces.output);
    });

    // Wait for both threads to finish
    obf_handle.join().expect("Obfuscating thread panicked");
    send_handle.join().expect("Sending thread panicked");
    deobf_handle.join().expect("Deobfuscating thread panicked");

    Ok(())
}

pub fn get_channel(interface_name: &str) -> Result<ChannelCustom, &'static str>{
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
        tx, 
        rx,
    };

    Ok(ch)
}

fn transmit(obf_output_interface: &str, rrs: Arc<Mutex<round_robin::RoundRobinScheduler>>) {
    let mut ch_tx = match get_channel(obf_output_interface) {
        Ok(tx) => tx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    // Keep track of time
    let interval = Duration::from_micros((1e6/PACKETS_PER_SECOND) as u64);
    //let interval = Duration::from_nanos(100);
    let mut last_iteration_time = Instant::now();

    // Send Ethernet frames
    loop {
        let mut scheduler = rrs.lock().unwrap();
        let packet = scheduler.pop();
        drop(scheduler);
        //println!("Transmit packet of length {}", packet.len());
        match ch_tx.tx.send_to(&packet, None) {
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

        // Calculate time to sleep
        let elapsed_time = last_iteration_time.elapsed();
        last_iteration_time = Instant::now();
        let sleep_time = if elapsed_time < interval {
            interval - elapsed_time
        } else {
            Duration::new(0, 0)
        };
        // Sleep for the remaining time until the next iteration
        thread::sleep(sleep_time);
    }
}

fn obfuscate<'a, 'b>(input_interface: &str, rrs: Arc<Mutex<round_robin::RoundRobinScheduler>>) 
where 'a: 'b {
    let mut ch_rx = match get_channel(input_interface) {
        Ok(rx) => rx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    // Process received Ethernet frames
    loop { 
        match ch_rx.rx.next() {
            // process_packet(packet, &mut scheduler),
            Ok(packet) =>  {
                //println!("Received length = {}", packet.len());
                let mut scheduler = rrs.lock().unwrap();
                scheduler.push(packet.to_vec());
                drop(scheduler);
            },
            Err(e) => {
                eprintln!("Error receiving frame: {}", e);
                continue;
            }
        };
    }
}

fn deobfuscate(obf_input_interface: &str, output_interface: &str) {
    let mut ch_rx = match get_channel(obf_input_interface) {
        Ok(rx) => rx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    let mut ch_tx = match get_channel(output_interface) {
        Ok(tx) => tx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    // Process received Ethernet frames
    loop {
        match ch_rx.rx.next() {
            // process_packet(packet, &mut scheduler),
            Ok(packet) =>  {
                match deobfuscate::process_packet(packet) {
                    // Real packets
                    Some(packet) => {
                        //println!("Deobfuscated packet with length = {}", packet.len());
                        ch_tx.tx.send_to(&packet, None);
                    }, 
                    // Chaff
                    None => continue,
                }
            },
            Err(e) => {
                eprintln!("Error receiving frame: {}", e);
                continue;
            }
        };
    }
}


