pub const QUERY_FOR_GAME_PACKET: [u8; 16] =
    [247, 47, 16, 0, 80, 88, 51, 87, 26, 0, 0, 0, 0, 0, 0, 0]; //TODO make game version configurable

pub fn update_port(packet: &mut [u8], new_port: u16) {
    let port_bytes = new_port.to_le_bytes();
    packet[packet.len() - 2] = port_bytes[0];
    packet[packet.len() - 1] = port_bytes[1];
}

pub fn hack_game_name(packet: &mut [u8], new_name: &[u8]) {
    for (offset, byte) in new_name.iter().enumerate() {
        packet[0x14 + offset] = *byte;
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Wc3UdpMessageType {
    QueryForGamesRequest,   //3a
    QueryForGamesResponse,  //3b
    NewServerHosted,        //3c
    NumberOfPlayersChanged, //3d
    ServerCanceled,         //3e
}

impl Wc3UdpMessageType {
    pub fn detect(packet: &[u8]) -> Option<Self> {
        let b0 = packet.get(0)?;
        let b1 = packet.get(1)?;

        match (b0, b1) {
            (0xF7, 0x2F) => Some(Wc3UdpMessageType::QueryForGamesRequest),
            (0xF7, 0x30) => Some(Wc3UdpMessageType::QueryForGamesResponse),
            (0xF7, 0x31) => Some(Wc3UdpMessageType::NewServerHosted),
            (0xF7, 0x32) => Some(Wc3UdpMessageType::NumberOfPlayersChanged),
            (0xF7, 0x33) => Some(Wc3UdpMessageType::ServerCanceled),
            _ => None,
        }
    }
}
