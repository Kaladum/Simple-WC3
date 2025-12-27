use std::str::FromStr;

use iroh::{EndpointAddr, PublicKey};

use crate::{
    client::demo_client,
    host::demo_host,
    utils::{APP_NAME, APP_VERSION},
};

mod client;
mod host;
mod packets;
mod utils;

#[tokio::main]
async fn main() {
    println!("{} V{}", APP_NAME, APP_VERSION);
    println!("Enter remote address to connect or press Enter to host:");
    let mut connect_to_remote = String::new();
    std::io::stdin()
        .read_line(&mut connect_to_remote)
        .expect("Failed to read address");
    connect_to_remote = connect_to_remote.trim().to_string();

    if connect_to_remote.is_empty() {
        println!("Starting as host");
        demo_host().await;
    } else {
        println!("Connecting to host");
        let address = PublicKey::from_str(&connect_to_remote).expect("Invalid address");
        demo_client(EndpointAddr::new(address)).await;
    }
}
