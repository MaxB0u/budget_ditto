use pnet::packet::ipv4;

// const ETHERNET_SRC_MAC_OFFSET: usize = 10;
// const MAC_ADDR_LEN: usize = 6;
const IP_HEADER_LEN: usize = 20;
// const IP_LEN_OFFSET: usize = 2;
const IP_SRC_ADDR_OFFSET: usize = 12;
const IP_ADDR_LEN: usize = 4;

enum PacketType {
    Chaff,          // Chaff -> All zeros. Look at byte after addresses (byte 13)
    Incoming,     // Obfuscated -> Other
    Outgoing,         // Normal -> N/A Only ditto traffic supported for now
}

fn get_packet_type(packet: &[u8], ip_src: [u8;4]) -> PacketType {
    // Get the type of packet, can be one of 3 options

    // Ethertype or id is never 0 byte except in chaff packets
    
    if packet[2] == 0_u8 && packet[3] == 0_u8 {
        return PacketType::Chaff;
    } else if packet[IP_SRC_ADDR_OFFSET..IP_SRC_ADDR_OFFSET+IP_ADDR_LEN] == ip_src {
        return PacketType::Outgoing;
    } else {
        return PacketType::Incoming;
    }
}

pub fn process_packet(packet: &[u8], ip_src: [u8;4]) -> Option<&[u8]> {
    let packet = unwrap_ipv4(packet);
    match get_packet_type(packet, ip_src) {
        PacketType::Chaff => None,
        PacketType::Outgoing => None,
        PacketType::Incoming => Some(deobfuscate(packet)),
        //_ => None
    }
}

fn deobfuscate(packet: &[u8]) -> &[u8] {
    // Or else it would be an invalid packet anyway
    assert!(packet.len() >= IP_HEADER_LEN, "Packet length must be at least {} bytes", IP_HEADER_LEN); 
    // let length: u16;

    // unsafe {
    //     let b1 = *packet.as_ptr().add(IP_LEN_OFFSET) as u16;
    //     let b2 = *packet.as_ptr().add(IP_LEN_OFFSET+1) as u16;
    //     length = (b1 << 8) | b2;
    // }

    // Try to get length, only support IP packets
    let pkt = ipv4::Ipv4Packet::new(packet).unwrap();
    let length= pkt.get_total_length();

    if length <= packet.len() as u16 {
        // println!("{}, {:?}", pkt.get_destination(), packet);
        &packet[..length as usize]
    } else {
        // println!("Failed to read length for packet of length {}. Read {}. Returned raw packet.", packet.len() as u16, length);
        // println!("{:?}", packet);
        //println!("{:?}, {:?}", packet[10], packet[11]);
        packet
    }

    
    
}

fn unwrap_ipv4(packet: &[u8]) -> &[u8] {
    assert!(packet.len() >= IP_HEADER_LEN, "Packet length must be at least {} bytes", IP_HEADER_LEN); 
    &packet[IP_HEADER_LEN..]
} 