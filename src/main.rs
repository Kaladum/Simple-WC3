use std::str::FromStr;

use iroh::{EndpointAddr, PublicKey};

use crate::{
    client::run_client,
    host::run_host,
    utils::{APP_NAME, APP_VERSION},
};

mod client;
mod host;
mod packets;
mod utils;

#[tokio::main]
async fn main() {
    println!("{} v{}", APP_NAME, APP_VERSION);
    println!("Visit https://github.com/Kaladum/Simple-WC3 for more information.");
    println!();
    println!("Enter remote address to connect or press Enter to host:");
    let mut connect_to_remote = String::new();
    std::io::stdin()
        .read_line(&mut connect_to_remote)
        .expect("Failed to read address");
    connect_to_remote = connect_to_remote.trim().to_string();

    if connect_to_remote.is_empty() {
        println!("Starting as host");
        run_host().await;
    } else {
        println!("Connecting to host");
        let address = PublicKey::from_str(&connect_to_remote).expect("Invalid address");
        run_client(EndpointAddr::new(address)).await;
    }
}
