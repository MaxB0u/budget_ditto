use pnet::packet::ipv4;

// const ETHERNET_SRC_MAC_OFFSET: usize = 10;
// const MAC_ADDR_LEN: usize = 6;
const IP_HEADER_LEN: usize = 20;
// const IP_LEN_OFFSET: usize = 2;
const IP_SRC_ADDR_OFFSET: usize = 12;
const IP_ADDR_LEN: usize = 4;

enum PacketType {
    Chaff,          // Chaff -> All zeros. Look at byte after addresses (byte 13)
    Obfuscated,     // Obfuscated -> Other
    // Normal,         // Normal -> N/A Only ditto traffic supported for now
}

fn get_packet_type(packet: &[u8]) -> PacketType {
    // Get the type of packet, can be one of 3 options

    // Ethertype or id is never 0 byte except in chaff packets
    
    if packet[2] == 0_u8 && packet[3] == 0_u8 {
        return PacketType::Chaff;
    } else {
        return PacketType::Obfuscated;
    }
}

pub fn process_packet(packet: &[u8], ip_src: [u8;4], is_local: bool) -> Option<&[u8]> {
    match unwrap_ipv4(packet, ip_src, is_local) {
        Some(packet) => {
            match get_packet_type(packet) {
                PacketType::Chaff => None,
                PacketType::Obfuscated => Some(deobfuscate(packet)),
                //_ => None
                }
            },
        None => None
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

fn unwrap_ipv4(packet: &[u8], ip_src: [u8;4], is_local: bool) -> Option<&[u8]> {
    // Unefficient since reecive all outgoing and incoming packets (more processing for no reason)
    // Better if could directly only receive incoming packets
    if packet[IP_SRC_ADDR_OFFSET..IP_SRC_ADDR_OFFSET+IP_ADDR_LEN] != ip_src && !is_local 
        || packet[IP_SRC_ADDR_OFFSET..IP_SRC_ADDR_OFFSET+IP_ADDR_LEN] == ip_src && is_local {
        // Src ip is the same if local and different if not
        assert!(packet.len() >= IP_HEADER_LEN, "Packet length must be at least {} bytes", IP_HEADER_LEN); 
        Some(&packet[IP_HEADER_LEN..])
    } else {
        // Outgoing packet
        None
    }
    
} 