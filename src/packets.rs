use binrw::{BinRead, BinWrite, NullString};

use crate::utils::try_parse;

#[derive(Debug)]
pub enum Wc3UdpMessageType {
    /** Packet 3a */
    QueryForGamesRequest,
    /** Packet 3b */
    QueryForGamesResponse(QueryForGamesResponse),
    /** Packet 3c */
    NewServerHosted,
    /** Packet 3d */
    NumberOfPlayersChanged,
    /** Packet 3e */
    ServerCanceled,
}

impl Wc3UdpMessageType {
    pub fn detect(packet: &[u8]) -> Option<Self> {
        let b0 = packet.get(0)?;
        let b1 = packet.get(1)?;
        match (b0, b1) {
            (0xF7, 0x2F) => Some(Wc3UdpMessageType::QueryForGamesRequest),
            (0xF7, 0x30) => Some(Wc3UdpMessageType::QueryForGamesResponse(try_parse::<
                QueryForGamesResponse,
            >(
                packet
            )?)),
            (0xF7, 0x31) => Some(Wc3UdpMessageType::NewServerHosted),
            (0xF7, 0x32) => Some(Wc3UdpMessageType::NumberOfPlayersChanged),
            (0xF7, 0x33) => Some(Wc3UdpMessageType::ServerCanceled),
            _ => None,
        }
    }
}

//Based on the implementation found at https://github.com/Qyperion/WC3LanGame
//There is also a Doc in that repo that describes the packet structure but it looks like the doc is wrong in some places.
#[derive(BinRead, BinWrite, Debug)]
#[brw(little)]
pub struct QueryForGamesResponse {
    #[br(assert(header == 0xF7))]
    pub header: u8, //byte 0
    #[br(assert(op_code == 0x30))]
    pub op_code: u8, //byte 1
    pub packet_size: u16,      //bytes 2-3
    pub game_type: GameType,   //bytes 4-7
    pub unknown1: u32,         //bytes 8-11
    pub game_id: u32,          //bytes 12-15
    pub unknown2: u32,         //bytes 16-19
    pub game_name: NullString, //bytes 20-? //looks like max 31 bytes + null terminator
    pub unknown3: u8,
    pub encoded: NullString,
    //pub encoded: Wc3Encoded,
    pub number_of_slots: u32,
    pub game_flags: u32,
    pub number_of_players: u32,
    pub number_of_player_slots: u32,
    pub game_age: u32, //just a guess
    pub tcp_port: u16,
}

#[derive(BinRead, BinWrite, Debug)]
#[brw(little)]
pub struct QueryForGamesRequest {
    #[br(assert(header == 0xF7))]
    pub header: u8, //byte 0
    #[br(assert(op_code == 0x2F))]
    pub op_code: u8, //byte 1
    #[br(assert(packet_size == 16))]
    pub packet_size: u16, //bytes 2-3
    pub game_type: GameType, //bytes 4-7
    pub game_version: u32,   //bytes 8-11
    #[br(assert(game_id == 0))] //Zero for request
    pub game_id: u32, //bytes 12-15
}

impl QueryForGamesRequest {
    pub fn new(game_type: GameType, game_version: u32) -> Self {
        QueryForGamesRequest {
            header: 0xF7,
            op_code: 0x2F,
            packet_size: 16,
            game_type,
            game_version,
            game_id: 0,
        }
    }
}

//The magic values are reversed for some reason.
#[derive(BinRead, BinWrite, Debug)]
#[brw(little)]
pub enum GameType {
    #[brw(magic = b"3RAW")] //WAR3
    Warcraft3,
    #[brw(magic = b"PX3W")] //W3XP
    TheFrozenThrone,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Wc3Encoded {
    pub values: QueryForGamesResponseInner,
}

impl BinRead for Wc3Encoded {
    type Args<'a> = u8;

    fn read_options<R: std::io::Read + std::io::Seek>(
        reader: &mut R,
        endian: binrw::Endian,
        _args: Self::Args<'_>,
    ) -> binrw::BinResult<Self> {
        let mut result = Vec::<u8>::new();

        loop {
            let byte = u8::read_options(reader, endian, ())?;
            if byte == 0 {
                break;
            }
            result.push(byte);
        }

        let decoded = decode_encoded_string(&result);
        let mut cursor = std::io::Cursor::new(&decoded);
        let values = QueryForGamesResponseInner::read(&mut cursor)?;

        Ok(Wc3Encoded { values })
    }
}

#[derive(BinRead, BinWrite, Debug)]
#[brw(little)]
pub struct QueryForGamesResponseInner {
    pub game_settings: u32, //Bytes 0-3
    pub unknown1: u8,       //Byte 4
    pub map_width: u16,     //Bytes 5-6
    pub map_height: u16,    //Bytes 7-8
    pub map_checksum: u32,  //Bytes 9-12
    pub map_name: NullString,
    pub host_username: NullString,
    pub unknown2: u8, //Byte 4
}

pub fn decode_encoded_string(encoded: &[u8]) -> Vec<u8> {
    let mut decoded = Vec::with_capacity(encoded.len());
    let mut mask = 0u8;
    for (i, &byte) in encoded.iter().enumerate() {
        if i % 8 != 0 {
            if (mask & (1 << (i % 8))) == 0 {
                decoded.push(byte - 1);
            } else {
                decoded.push(byte);
            }
        } else {
            mask = byte;
        }
    }
    decoded
}
