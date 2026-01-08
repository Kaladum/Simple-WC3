use std::{
    io::Cursor,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    ops::RangeInclusive,
};

use binrw::{
    BinRead, BinWrite,
    meta::{ReadEndian, WriteEndian},
};

use crate::packets::GameType;

pub const APP_NAME: &str = "Simple-WC3";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const ALPN: &[u8] = concat!("simple-wc3-", env!("CARGO_PKG_VERSION")).as_bytes();

pub const WC3_DEFAULT_PORT: u16 = 6112;
pub const ZERO_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
pub const ZERO_SOCKET_ADDR: SocketAddr = SocketAddr::new(ZERO_IP, 0);

pub const LOCALHOST_V4: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
pub const LOCALHOST_WC3_ADDR: SocketAddr = SocketAddr::new(LOCALHOST_V4, WC3_DEFAULT_PORT);

pub fn try_parse<T: BinRead + ReadEndian>(data: &[u8]) -> Option<T>
where
    for<'a> <T as BinRead>::Args<'a>: Default,
{
    let mut cursor = Cursor::new(data);
    T::read(&mut cursor).ok()
}

pub fn try_serialize<T: BinWrite + WriteEndian>(value: &T) -> Option<Vec<u8>>
where
    for<'a> <T as BinWrite>::Args<'a>: Default,
{
    let mut serialized = Vec::new();
    let mut cursor = Cursor::new(&mut serialized);
    value.write(&mut cursor).ok()?;
    Some(serialized)
}

pub const SUPPORTED_GAME_VERSIONS: RangeInclusive<u32> = 25..=31;
pub const SUPPORTED_GAME_TYPES: [GameType; 2] = [GameType::Warcraft3, GameType::TheFrozenThrone];
