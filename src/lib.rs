pub mod pattern;
mod deobfuscate;
pub mod queues;
mod feature_flags;
pub mod hardware_obf;

use std::fs::OpenOptions;
use std::io::Write;
use std::net;
use crate::queues::round_robin;
use pnet::datalink;
use pnet::datalink::Channel::Ethernet;
use std::error::Error;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use toml::Value;

const FACTOR_MEGABITS: f64 = 1e6;
const BITS_PER_BYTE: f64 = 8.0;

pub struct ChannelCustom {
    pub tx: Box<dyn datalink::DataLinkSender>,
    pub rx: Box<dyn datalink::DataLinkReceiver>,
    pub mac_addr: Option<pnet::util::MacAddr>,
}

pub fn run(settings: Value) -> Result<(), Box<dyn Error>> {
    
    let rate = settings["general"]["rate"].as_float().expect("Rate setting not found");
    let pps = rate / pattern::get_average_pattern_length() * FACTOR_MEGABITS / BITS_PER_BYTE;
    // println!("{}", pps);

    let pad_log_interval = match settings["general"]["pad_log_interval"].as_float()  {
        Some(p) => p,
        None => 0.1*pps,
    };
    
    let save_data = settings["general"]["save"].as_bool().expect("Save setting not found");
    let is_local = settings["general"]["local"].as_bool().expect("Is local setting not found");
    let is_log = settings["general"]["log"].as_bool().expect("Is log setting not found");
    let is_hw_obfuscation = settings["general"]["hw_obfuscation"].as_bool().expect("Obfuscation Mode setting not found");
    let is_backbone = settings["general"]["backbone"].as_bool().expect("Is backbone setting not found");

    let avg_pkt_size = pattern::PATTERN.iter().sum::<usize>() as f64 / pattern::PATTERN.len() as f64;
    println!("Sending {} packets/s with avg size of {}B => rate = {:.2} KB/s", pps, avg_pkt_size, pps*avg_pkt_size/1000.0);

    let ip_src = parse_ip(settings["ip"]["src"].as_str().expect("Src ip address not found").to_string());
    let ip_dst = parse_ip(settings["ip"]["dst"].as_str().expect("Dst ip address not found").to_string());

    println!("Setting up queues for pattern {:?}", pattern::PATTERN);
    let rrs = Arc::new(round_robin::RoundRobinScheduler::new(pattern::PATTERN.len(), pps, ip_src, ip_dst));

    let tx_queue = Arc::clone(&rrs);
    let rx_queue = Arc::clone(&rrs);

    let is_deobf_isolated = settings["isolation"]["isolate_deobfuscate"].as_bool().expect("Isolate deobf setting not found");
    let core_id_deobf = settings["isolation"]["core_deobfuscate"].as_integer().expect("Core deobf setting not found") as usize;

    let is_send_isolated = settings["isolation"]["isolate_send"].as_bool().expect("Isolate send setting not found");  
    let core_id_send = settings["isolation"]["core_send"].as_integer().expect("Core send setting not found") as usize;

    let is_obf_isolated = settings["isolation"]["isolate_obfuscate"].as_bool().expect("Isolate obf setting not found");     
    let core_id_obf = settings["isolation"]["core_obfuscate"].as_integer().expect("Core obf setting not found") as usize;

    let priority = settings["isolation"]["priority"].as_integer().expect("Thread priority setting not found") as i32; 

    let interface_obfuscate = settings["interface"]["no_obf"].as_str().expect("Unobfuscated interface setting not found").to_string(); 
    let interface_transmit: String = settings["interface"]["obf"].as_str().expect("Obfuscated interface setting not found").to_string(); 
    let interface_deobfuscate_input = settings["interface"]["obf"].as_str().expect("Unobfuscated interface setting not found").to_string(); 
    let interface_deobfuscate_output: String = settings["interface"]["no_obf"].as_str().expect("Obfuscated interface setting not found").to_string(); 
    let src_device: String = settings["interface"]["src_device"].as_str().expect("Obfuscated interface setting not found").to_string(); 

    if is_log {
        println!("Listening for Ethernet frames on interface {}...", interface_obfuscate);
        println!("Sending obfuscated Ethernet frames on interface {}...", interface_transmit);
        println!("Listening for obfuscated Ethernet frames on interface {}...", interface_deobfuscate_input);
        println!("Sending deobfuscated Ethernet frames on interface {}...", interface_deobfuscate_output);
        println!("Send on specific cores = {}", is_send_isolated);
        println!("Using hardware obfuscation = {}", is_hw_obfuscation);
        println!("Running as a backbone router = {}", is_backbone);
    }

    // Spawn thread for obfuscating packets
    let obf_handle = thread::spawn(move || {
        if is_obf_isolated {
            unsafe {
                let mut cpuset: libc::cpu_set_t = std::mem::zeroed();
                libc::CPU_SET(core_id_obf, &mut cpuset);
                libc::sched_setaffinity(0, std::mem::size_of_val(&cpuset), &cpuset);

                let thread =  libc::pthread_self();
                let param = libc::sched_param { sched_priority: priority };
                let result = libc::pthread_setschedparam(thread, libc::SCHED_FIFO, &param as *const libc::sched_param);
                if result != 0 {
                    panic!("Failed to set thread priority");
                }
            }
        }
        if feature_flags::FF_NO_REORDERING {
            obfuscate_data_in_order(&interface_obfuscate, rx_queue, pps, pad_log_interval, save_data);
        } else {
            obfuscate_data(&interface_obfuscate, &src_device, rx_queue, pps, pad_log_interval, save_data);
        }
    });

    // Spawn thread for sending obfuscated packets
    let send_handle = thread::spawn(move || {
        if is_send_isolated {
            unsafe {
                let mut cpuset: libc::cpu_set_t = std::mem::zeroed();
                libc::CPU_SET(core_id_send, &mut cpuset);
                libc::sched_setaffinity(0, std::mem::size_of_val(&cpuset), &cpuset);

                let thread =  libc::pthread_self();
                let param = libc::sched_param { sched_priority: priority };
                let result = libc::pthread_setschedparam(thread, libc::SCHED_FIFO, &param as *const libc::sched_param);
                if result != 0 {
                    panic!("Failed to set thread priority");
                }
            }
        }

        transmit(&interface_transmit, tx_queue, pps, save_data);
    });

    // Spawn thread for sending deobfuscating and forwarding packets
    let deobf_handle = thread::spawn(move || {
        if is_deobf_isolated {
            unsafe {
                let mut cpuset: libc::cpu_set_t = std::mem::zeroed();
                libc::CPU_SET(core_id_deobf, &mut cpuset);
                libc::sched_setaffinity(0, std::mem::size_of_val(&cpuset), &cpuset);
            }
        }

        deobfuscate_data(&interface_deobfuscate_input, &interface_deobfuscate_output, ip_src, is_local, is_hw_obfuscation, is_backbone);
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
    
    let mac_addr = interface.mac;

    // Create a channel to receive Ethernet frames
    let (tx, rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => return Err("Unknown channel type"),
        Err(e) => panic!("Failed to create channel {e}"),
    };

    let ch = ChannelCustom{ 
        tx, 
        rx,
        mac_addr,
    };

    Ok(ch)
}

fn transmit(obf_output_interface: &str, rrs: Arc<round_robin::RoundRobinScheduler>, pps: f64, save_data: bool) {
    println!("Transmitting data...");

    let mut ch_tx = match get_channel(obf_output_interface) {
        Ok(tx) => tx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    // Keep track of time
    let interval = Duration::from_nanos((1e9/pps) as u64);
    println!("Sending packets in intervals of {:?}", interval);

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(save_data) // Overwrite
        .create(true)
        .open("data.csv")
        .expect("Could not open file");

    if save_data {
        writeln!(file, "Iteration,Time").expect("Failed to write to file");

        write_params_to_file(save_data, interval.as_nanos());
    }
    
    //let interval = Duration::from_nanos(100);
    let mut current_q = 0;
    // let mut count: usize = 0;
    // let mut delays = vec![0; 2e6 as usize];
    // Send Ethernet frames
    // for _ in 0..NUM_PKTS_TO_SAVE as usize {

    let mut last_iteration_time = Instant::now();
    loop {
        let packet = rrs.pop(current_q);
        current_q = (current_q + 1) % pattern::PATTERN.len();

        // println!("Transmit packet of length {}", packet.len());
        match ch_tx.tx.send_to(&packet, None) {
            Some(res) => {
                match res {
                    Ok(_) => (),
                    Err(e) => println!("Error sending frame: {}", e),
                }
            }
            None => {
                println!("No packets to send");
            }
        }

        // Calculate time to sleep
        let elapsed_time = last_iteration_time.elapsed();
        let sleep_time = if elapsed_time < interval {
            interval - elapsed_time
        } else {
            Duration::new(0, 0)
        };
        // Sleep for the remaining time until the next iteration
        thread::sleep(sleep_time);
        if elapsed_time > interval {
            // println!("Ran out of time processing {:?} at pkt {}", elapsed_time, count);
        }
        last_iteration_time = last_iteration_time + interval;
        
        // if save_data {
        //     let elapsed_time = last_iteration_time.elapsed();
        //     //writeln!(file, "{},{}", count, elapsed_time.as_nanos()).expect("Failed to write to file");
        //     delays[count] = elapsed_time.as_nanos()
        // }
        // count += 1;
    }

    // println!("Done trasnmitting data!");

    // if save_data {
    //     println!("Saving...");
    //     for i in 0..delays.len() {
    //         writeln!(file, "{},{}", i, delays[i]).expect("Failed to write to file");
    //     }
    //     println!("Data saved to file!");
    // }
}

fn obfuscate_data(input_interface: &str, src_device: &str, rrs: Arc<round_robin::RoundRobinScheduler>, pps: f64, pad_log_interval: f64, save_data: bool) {
    let mut ch_rx = match get_channel(input_interface) {
        Ok(rx) => rx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    let ch_src = match get_channel(src_device) {
        Ok(rx) => rx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(save_data) // Overwrite
        .create(true)
        .open("pad.csv")
        .expect("Could not open file");

    if save_data {
        writeln!(file, "Iteration,Pad").expect("Failed to write to file");
    }

    let mut count = 0;
    let mut psv = pattern::get_push_state_vector();
    let mac_addr = ch_rx.mac_addr.unwrap();
    let src_mac = ch_src.mac_addr.unwrap();
    // println!("src mac: {:?} or {:?}", mac_addr, src_mac);
    // Process received Ethernet frames
    loop { 
        match ch_rx.rx.next() {
            // process_packet(packet, &mut scheduler),
            Ok(packet) =>  {
                if check_src_eth(packet, mac_addr, src_mac) {
                    // let pkt_len = packet.len();
                    // println!("Received length = {}", packet.len());
                    let idx = rrs.push(packet.to_vec(), &psv);
                    let mut previous_state = 0;
                    if idx == pattern::PATTERN.len() {
                        // println!("Failed to push packet of length {}", pkt_len);
                        continue;
                    } else if idx > 0 {
                        previous_state = psv[idx-1].1;
                    } 
                    // println!("Pushed packet to queue {}", idx);
                    // println!("Pushed packet of length {}", packet.len());
                    // We pushed in a state with many queues, adjust the next queue that will be pushed to in that state
                    let modulus = psv[idx].1 - previous_state;
                    let next_queue = psv[idx].0 - previous_state + 1;
                    psv[idx].0 = next_queue % modulus + previous_state;
                }
            },
            Err(e) => {
                eprintln!("Error receiving frame: {}", e);
                continue;
            }
        };

        if count % pad_log_interval as usize == 0 && count != 0{
            let lock_pad = round_robin::TOTAL_PAD.lock().unwrap();
            // Could reset it here if want to or else moving average
            let avg_pad = (*lock_pad) / count as f64 * pps;
            if save_data {
                writeln!(file, "{},{}", count, avg_pad).expect("Failed to write to file");
            } else {
                // println!("Average pad of {:.2}B", avg_pad);
            }
        }

        count += 1;
    }
}

fn deobfuscate_data(obf_input_interface: &str, output_interface: &str, ip_src: [u8;4], is_local: bool, is_hw_obfuscation: bool, is_backbone: bool) {
    let mut ch_rx = match get_channel(obf_input_interface) {
        Ok(rx) => rx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    let mut ch_tx = match get_channel(output_interface) {
        Ok(tx) => tx,
        Err(error) => panic!("Error getting channel: {error}"),
    };


    let mac_addr = ch_tx.mac_addr.unwrap().octets();
    // println!("CHange mac to {:?}", mac_addr);

    // Process received Ethernet frames
    loop {
        match ch_rx.rx.next() {
            // process_packet(packet, &mut scheduler),
            Ok(packet) =>  {
                match deobfuscate::process_packet(&packet, ip_src, is_local, is_hw_obfuscation) {
                    // Real packets
                    Some(packet) => {
                        // println!("Deobfuscated packet with length = {}", packet.len());
                        if is_backbone {
                            ch_tx.tx.send_to(&process_backbone_packet(packet, mac_addr), None);
                        } else {
                            ch_tx.tx.send_to(&packet, None);
                        }
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

fn parse_ip(ip_str: String) -> [u8;4] {
    let ip_addr = match ip_str.parse::<net::Ipv4Addr>() {
        Ok(addr) => addr,
        Err(e) => {
            panic!("Failed to parse IP address: {}", e);
        }
    };
    ip_addr.octets()
}

fn write_params_to_file<T: std::fmt::Display>(overwrite: bool, interval: T) {
    let mut params_file = OpenOptions::new()
            .write(true)
            .truncate(overwrite) // Overwrite
            .create(true)
            .open("parameters.csv")
            .expect("Could not open file");

    writeln!(params_file, "Name,Value").expect("Failed to write to file");
    writeln!(params_file, "interval,{}",interval).expect("Failed to write to file");
    writeln!(params_file, "pattern, {:?}", pattern::PATTERN).expect("Failed to write to file");
}

fn obfuscate_data_in_order(input_interface: &str, rrs: Arc<round_robin::RoundRobinScheduler>, pps: f64, pad_log_interval: f64, save_data: bool) {
    let mut ch_rx = match get_channel(input_interface) {
        Ok(rx) => rx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(save_data) // Overwrite
        .create(true)
        .open("pad.csv")
        .expect("Could not open file");

    if save_data {
        writeln!(file, "Iteration,Pad").expect("Failed to write to file");
    }

    let mut count = 0;
    let mut current_q = 0;
    let mac_addr = ch_rx.mac_addr.unwrap();
    println!("src mac: {:?}", mac_addr);
    // Process received Ethernet frames
    loop { 
        match ch_rx.rx.next() {
            // process_packet(packet, &mut scheduler),
            Ok(packet) =>  {
                //println!("Received length = {}", packet.len());
                if check_src_eth(packet, mac_addr, mac_addr) {
                    current_q = rrs.push_no_reorder(packet.to_vec(), current_q);
                }
            },
            Err(e) => {
                eprintln!("Error receiving frame: {}", e);
                continue;
            }
        };

        if count % pad_log_interval as usize == 0 && count != 0{
            let lock_pad = round_robin::TOTAL_PAD.lock().unwrap();
            // COuld reset it here if want to or else moving average
            let avg_pad = (*lock_pad) / count as f64 * pps;
            if save_data {
                writeln!(file, "{},{}", count, avg_pad).expect("Failed to write to file");
            } else {
                // println!("Average pad of {:.2}B", avg_pad);
            }
        }

        count += 1;
    }
}

fn check_src_eth(data: &[u8], mac_addr: pnet::util::MacAddr, src_device_mac: pnet::util::MacAddr) -> bool {
    let packet = pnet::packet::ethernet::EthernetPacket::new(data).unwrap();
    // println!("{}", packet.get_source());
    let data_mac = packet.get_source();

    data_mac == mac_addr || data_mac == src_device_mac
}

fn process_backbone_packet(packet: &[u8], mac_addr: [u8; 6]) -> Vec<u8> {
    // Set ip dst and mac for deobfuscated packets that should be forwarded
    // assume the destination is zurich and the destination ip address is already in the 10.7.0.0/24 subnet
    let mut pkt = vec![0u8; packet.len()]; 
    pkt.clone_from_slice(packet);

    pkt[pattern::IP_HEADER_LEN+pattern::ETH_MAC_SRC_ADDR_OFFSET.. pattern::IP_HEADER_LEN+pattern::ETH_MAC_SRC_ADDR_OFFSET+pattern::MAC_ADDR_LEN]
        .copy_from_slice(&mac_addr);
    
    pkt[pattern::IP_HEADER_LEN+pattern::ETH_HEADER_LEN+pattern::IP_DST_ADDR_OFFSET..pattern::IP_HEADER_LEN+pattern::ETH_HEADER_LEN+pattern::IP_DST_ADDR_OFFSET+pattern::IP_ADDR_LEN]
        .copy_from_slice(&pattern::IP_NEXT_HOP);
    pkt
}
