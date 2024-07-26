use pnet::packet::ethernet;

const ETHERTYPE_PADDING_META: u16 = 2184; 
const PADDING_META_LEN: usize = 18; // Length is 18 Bytes (or 144 bits)

const ETHERTYPE_32B_PADS: u16 = 2049;
const ETHERTYPE_16B_PADS: u16 = 2050;
const ETHERTYPE_8B_PADS: u16 = 2051;
const ETHERTYPE_4B_PADS: u16 = 2052;
const ETHERTYPE_2B_PADS: u16 = 2053;
const ETHERTYPE_1B_PADS: u16 = 9; 

const ETHERTYPE_1B_PADS_TWO_TIMES_IN_A_ROW: u16 = 2313;
const ETHERTYPE_LAST_PAD: u16 = 2304;


pub fn deobfuscate_tofino(eth_buff: &[u8]) -> &[u8] {

    let eth_pkt = ethernet::EthernetPacket::new(eth_buff).unwrap();

    // println!("{}, {}", eth_pkt.get_ethertype().0, eth_buff.len());

    // Problem, when look at ethertype, it is 8B for a 1B padding
    // When recirculate, seem to have 46B next queue header with ethertype at bytes 45,46
    // Right now when I see that I cut 32B so it is as if the remaining 14B are a valid header
    // However, it does not remove the last byte

    match eth_pkt.get_ethertype().0 {
        ETHERTYPE_PADDING_META => deobfuscate_tofino(&eth_buff[PADDING_META_LEN..]),
        ETHERTYPE_32B_PADS => deobfuscate_tofino(&eth_buff[32..]),
        ETHERTYPE_16B_PADS => deobfuscate_tofino(&eth_buff[16..]),
        ETHERTYPE_8B_PADS => deobfuscate_tofino(&eth_buff[8..]),
        ETHERTYPE_4B_PADS => deobfuscate_tofino(&eth_buff[4..]),
        ETHERTYPE_2B_PADS => deobfuscate_tofino(&eth_buff[2..]),
        ETHERTYPE_1B_PADS => deobfuscate_tofino(&eth_buff[1..]),
        ETHERTYPE_1B_PADS_TWO_TIMES_IN_A_ROW => deobfuscate_tofino(&eth_buff[1..]),
        ETHERTYPE_LAST_PAD => {
            if eth_buff[14] == 69 {
                // Start of ipv4 frame, return that
                return &eth_buff[14..];
            }
            deobfuscate_tofino(&eth_buff[32..])
        },
        _ => &eth_buff[14..]
    }
}