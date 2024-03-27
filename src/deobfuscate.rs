const ETHERNET_SRC_MAC_OFFSET: usize = 10;

enum PacketType {
    Chaff,          // Chaff -> All zeros. Look at byte after addresses (byte 13)
    Obfuscated,     // Obfuscated -> Other
    //Normal,         // Normal -> N/A Only ditto traffic supported for now
}

fn get_packet_type(packet: &[u8]) -> PacketType {
    // Get the type of packet, can be one of 3 options

    // Ethertype or id never starts with 0 byte except in chaff packets
    if packet[12] == 0_u8 {
        return PacketType::Chaff;
    } else {
        return PacketType::Obfuscated;
    }
}

pub fn process_packet(packet: &[u8]) -> Option<&[u8]> {
    match get_packet_type(packet) {
        PacketType::Chaff => None,
        PacketType::Obfuscated => Some(deobfuscate(packet)),
        //_ => None
    }
}

fn deobfuscate(packet: &[u8]) -> &[u8] {
    // Or else it would be an invalid packet anyway
    assert!(packet.len() >= 6, "Packet length must be at least 6 bytes"); 
    let length: u16;

    unsafe {
        let ptr = packet.as_ptr().add(ETHERNET_SRC_MAC_OFFSET) as *const u16;
        length = *ptr;
    }

    &packet[..length as usize]
}