pub mod pattern;
mod deobfuscate;
pub mod queues;
mod feature_flags;

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
//use libc::{c_int, c_void, size_t, sockaddr_ll, socket, AF_PACKET, SOCK_RAW, SOL_PACKET, PACKET_OUTGOING, sendto};

// Rate is Packet/s * Bytes/packet
//pub const PACKETS_PER_SECOND: f64 = 1e4; // 1e4 -> 100micros between packets

pub struct ChannelCustom {
    pub tx: Box<dyn datalink::DataLinkSender>,
    pub rx: Box<dyn datalink::DataLinkReceiver>,
}

pub fn run(settings: Value) -> Result<(), Box<dyn Error>> {
    
    // let devices = Device::list()?;
    // for device in &devices {
    //     println!("Device: {}", device.name);
    // }
    let pps = settings["general"]["pps"].as_float().expect("PPS setting not found");
    let save_data = settings["general"]["save"].as_bool().expect("Save setting not found");
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

    let input = settings["interface"]["input"].as_str().expect("Input interface setting not found").to_string(); 
    let obfuscate = settings["interface"]["obfuscate"].as_str().expect("Obf output interface setting not found").to_string(); 
    let deobfuscate = settings["interface"]["deobfuscate"].as_str().expect("Obf input interface setting not found").to_string(); 
    let output = settings["interface"]["output"].as_str().expect("Output interface setting not found").to_string(); 

    println!("Listening for Ethernet frames on interface {}...", input);
    println!("Sending obfuscated Ethernet frames on interface {}...", obfuscate);
    println!("Listening for obfuscated Ethernet frames on interface {}...", deobfuscate);
    println!("Sending deobfuscated Ethernet frames on interface {}...", output);


    println!("Send on specific cores = {}", is_send_isolated);

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
            obfuscate_data_in_order(&input, rx_queue, pps);
        } else {
            obfuscate_data(&input, rx_queue, pps);
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

        transmit(&obfuscate, tx_queue, pps, save_data);
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

        deobfuscate_data(&deobfuscate, &output, ip_src);
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

fn transmit(obf_output_interface: &str, rrs: Arc<round_robin::RoundRobinScheduler>, pps: f64, save_data: bool) {
    let mut ch_tx = match get_channel(obf_output_interface) {
        Ok(tx) => tx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    // Keep track of time
    let interval = Duration::from_nanos((1e9/pps) as u64);

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
    let mut count: usize = 0;
    let mut delays = vec![0; 2e6 as usize];
    // Send Ethernet frames
    // for i in 0..2e6 as usize {
    loop {
        let last_iteration_time = Instant::now();
        let packet = rrs.pop(current_q);
        current_q = (current_q + 1) % pattern::PATTERN.len();

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

        // println!("Transmit packet of length {}", packet.len());
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
        count += 1;

        if save_data {
            let elapsed_time = last_iteration_time.elapsed();
            //writeln!(file, "{},{}", count, elapsed_time.as_nanos()).expect("Failed to write to file");
            delays[count] = elapsed_time.as_nanos()
        }
    }

    // if save_data {
    //     for i in 0..delays.len() {
    //         writeln!(file, "{},{}", i, delays[i]).expect("Failed to write to file");
    //     }
    // }
}

fn obfuscate_data(input_interface: &str, rrs: Arc<round_robin::RoundRobinScheduler>, pps: f64) {
    let mut ch_rx = match get_channel(input_interface) {
        Ok(rx) => rx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    let mut count = 0;
    // Process received Ethernet frames
    loop { 
        match ch_rx.rx.next() {
            // process_packet(packet, &mut scheduler),
            Ok(packet) =>  {
                //println!("Received length = {}", packet.len());
                rrs.push(packet.to_vec());
            },
            Err(e) => {
                eprintln!("Error receiving frame: {}", e);
                continue;
            }
        };
        count += 1;
        if count % pps as usize == 0 {
            //let lock_pad = round_robin::TOTAL_PAD.lock().unwrap();
            //let avg_pad = (*lock_pad) / count as f64 * pps;
            //println!("Average pad of {:.2}B", avg_pad);
        }
    }
}

fn deobfuscate_data(obf_input_interface: &str, output_interface: &str, ip_src: [u8;4]) {
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
                match deobfuscate::process_packet(packet, ip_src) {
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

// pub fn get_env_var_f64(name: &str) -> Result<f64, &'static str> {
//     let var = match env::var(name) {
//         Ok(var) => {
//             match var.parse::<f64>() {
//                 Ok(var) => {
//                     var
//                 },
//                 Err(_) => {
//                     return Err("Error parsing env variable string");
//                 }
//             }
//         },
//         Err(_) => {
//             return Err("Error getting env vairable");
//         },
//     };
//     Ok(var)
// }

fn parse_ip(ip_str: String) -> [u8;4] {
    let ip_addr = match ip_str.parse::<net::Ipv4Addr>() {
        Ok(addr) => addr,
        Err(e) => {
            panic!("Failed to parse IP address: {}", e);
        }
    };
    ip_addr.octets()
}

fn write_params_to_file<T: std::fmt::Display>(overwrite: bool, param: T) {
    let mut params_file = OpenOptions::new()
            .write(true)
            .truncate(overwrite) // Overwrite
            .create(true)
            .open("parameters.csv")
            .expect("Could not open file");

        writeln!(params_file, "Name,Value").expect("Failed to write to file");
        writeln!(params_file, "interval,{}",param).expect("Failed to write to file");
}

fn obfuscate_data_in_order(input_interface: &str, rrs: Arc<round_robin::RoundRobinScheduler>, pps: f64) {
    let mut ch_rx = match get_channel(input_interface) {
        Ok(rx) => rx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    let mut count = 0;
    let mut current_q = 0;
    // Process received Ethernet frames
    loop { 
        match ch_rx.rx.next() {
            // process_packet(packet, &mut scheduler),
            Ok(packet) =>  {
                //println!("Received length = {}", packet.len());
                current_q = rrs.push_no_reorder(packet.to_vec(), current_q);
            },
            Err(e) => {
                eprintln!("Error receiving frame: {}", e);
                continue;
            }
        };

        count += 1;
        if count % pps as usize == 0 {
            //let lock_pad = round_robin::TOTAL_PAD.lock().unwrap();
            //let avg_pad = (*lock_pad) / count as f64 * pps;
            //println!("Average pad of {:.2}B", avg_pad);
        }
    }
}

// unsafe fn send_to_faster(data: &[u8]) {
//     // Create a raw socket
//     let sock_fd = unsafe {
//         socket(AF_PACKET, SOCK_RAW, 0)
//     };
//     if sock_fd == -1 {
//         panic!("Failed to create socket");
//     }

//     // Prepare destination address
//     let ifindex: c_int = 1; // Example interface index
//     let mut sockaddr = sockaddr_ll {
//         sll_family: AF_PACKET as u16,
//         sll_protocol: 0, // Set protocol if needed
//         sll_ifindex: ifindex,
//         sll_hatype: 0,
//         sll_pkttype: 0,
//         sll_halen: 0,
//         sll_addr: [0; 8], // Set MAC address if needed
//     };

//     // Call sendto
//     let result = unsafe {
//         sendto(
//             sock_fd,
//             data.as_ptr() as *const c_void,
//             data.len(),
//             0,
//             &mut sockaddr as *mut sockaddr_ll as *mut _,
//             std::mem::size_of::<sockaddr_ll>() as u32,
//         )
//     };

//     if result == -1 {
//         panic!("sendto failed");
//     } 
// }

// unsafe fn low_level_send(pkt: &[u8]) -> Result<(), Box<dyn Error>> {

//     // Send the frame
//     let result = sendto(
//         sockfd,
//         frame_data.as_ptr() as *const c_void,
//         frame_data.len(),
//         0,
//         &dest_addr as *const sockaddr_ll as *const sockaddr,
//         std::mem::size_of::<sockaddr_ll>() as u32,
//     );

//     // Check for errors
//     if result < 0 {
//         Err(io::Error::last_os_error())
//     } else {
//         Ok(())
//     }
// }