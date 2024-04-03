use budget_ditto;
use budget_ditto::queues::round_robin;
use std::thread;
use pnet::packet::ethernet;
use pnet::util::MacAddr;
use rand::prelude::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

const NUM_PACKETS: f64 = 1e3;
const MIN_ETH_LEN: i32 = 64;
const MTU: usize = 1500;
const EMPTY_PKT: [u8; MTU] = [0; MTU];

fn send(input: &str) {
    let mut ch_tx = match budget_ditto::get_channel(input) {
        Ok(tx) => tx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    let packets = get_eth_frames();

    for i in 0..NUM_PACKETS as usize {
        match ch_tx.tx.send_to(&packets[i], None) {
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

fn receive(output: &str) {
    let mut ch_rx = match budget_ditto::get_channel(output) {
        Ok(rx) => rx,
        Err(error) => panic!("Error getting channel: {error}"),
    };

    thread::spawn(move || {
        for _ in 0..NUM_PACKETS as usize {
            match ch_rx.rx.next() {
                // process_packet(packet, &mut scheduler),
                Ok(_) =>  {
                    //println!("Received length = {}", packet.len());
                },
                Err(e) => {
                    eprintln!("Error receiving frame: {}", e);
                    continue;
                }
            };
        }
    });
}

fn get_eth_frames() -> Vec<Vec<u8>>{
    let src_mac = MacAddr::new(0x05, 0x04, 0x03, 0x02, 0x01, 0x00);
    let dst_mac = MacAddr::new(0x00, 0x01, 0x02, 0x03, 0x04, 0x05);
    let mut frame_buff: Vec<Vec<u8>> = Vec::new();
    for _ in 0..NUM_PACKETS as i32 {
        let length = get_random_pkt_len() as usize;
        let mut eth_buff = EMPTY_PKT[0..length].to_vec();
        let mut eth_pkt = ethernet::MutableEthernetPacket::new(&mut eth_buff).unwrap();
        eth_pkt.set_source(src_mac);
        eth_pkt.set_destination(dst_mac);
        eth_pkt.set_ethertype(ethernet::EtherType::new(length as u16));

        frame_buff.push(eth_buff);
    }
    //println!("{:?}", frame_buff[0]);
    frame_buff
}

fn get_random_pkt_len() -> i32 {
    let mut rng = rand::thread_rng();
    rng.gen_range(MIN_ETH_LEN..=MTU as i32)
}

fn rr_push() {
    let pkts = get_eth_frames();
    let mut rrs = round_robin::RoundRobinScheduler::new(budget_ditto::pattern::PATTERN.len());
    for p in pkts {
        rrs.push(p);
    }
}

fn bench_send(c: &mut Criterion) {
    let input = "eth1";
    c.bench_function("send", |b| b.iter(|| send(black_box(input))));
}

fn bench_get_pkts(c: &mut Criterion) {
    c.bench_function("get_eth_frames", |b| b.iter(|| get_eth_frames()));
}

fn bench_get_channel(c: &mut Criterion) {
    let input = "eth1";
    c.bench_function("get_channel", |b| b.iter(|| budget_ditto::get_channel(black_box(input))));
}

fn bench_channel(c: &mut Criterion) {
    let input = "eth2";
    c.bench_function("receive", |b| b.iter(|| receive(black_box(input))));
}

fn bench_receive(c: &mut Criterion) {
    let input = "eth3";
    c.bench_function("receive", |b| b.iter(|| receive(black_box(input))));
}

fn bench_rr_push(c: &mut Criterion) {
    c.bench_function("rr_push", |b| b.iter(|| rr_push()));
}

// Before running this need to setup virtual eth 1,2,3
// And to urn ditto in another window with command
// sudo -E cargo run eth1 eth2 eth2 eth3
// to run it all on same device
criterion_group!(tx, bench_send);
criterion_group!(gen, bench_get_pkts); // 8 micro sec
criterion_group!(ch, bench_channel);
criterion_group!(rx, bench_receive);
criterion_group!(get_ch, bench_get_channel);
criterion_group!(push, bench_rr_push);
// benchmark_main!(tx, ch, rx);
// criterion_main!(get_ch, gen, tx, ch, rx, push);
criterion_main!(gen, push);