use iroh::{
    Endpoint,
    endpoint::{Connection, WriteError},
    protocol::{AcceptError, ProtocolHandler, Router},
};
use tokio::{
    net::TcpStream,
    sync::broadcast::{Receiver, Sender},
};

use crate::{
    game_scanner,
    packets::GenerableWc3UdpMessageType,
    utils::{ALPN, LOCALHOST_WC3_ADDR, try_serialize},
};

pub async fn run_host() {
    let ep = Endpoint::builder()
        .bind()
        .await
        .expect("Can't create endpoint");

    let game_scanner_tx = game_scanner::run_game_scanner().await;

    let handler = ClientHandler {
        scanner: game_scanner_tx,
    };
    Router::builder(ep.clone()).accept(ALPN, handler).spawn();
    ep.online().await;
    println!("Host is running with address:");
    println!("{}", ep.addr().id);
    println!();
    println!(
        "Copy this address (by selecting it and right-clicking) and share it with all players to let them connect"
    );
    println!("Press Ctrl+C or close the window to shut down");
    println!();

    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for event");
    println!("Shutting down host...");
}

#[derive(Debug, Clone)]
struct ClientHandler {
    pub scanner: Sender<GenerableWc3UdpMessageType>,
}

impl ProtocolHandler for ClientHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        println!("New client connected: {}", connection.remote_id());

        let scanner = self.scanner.subscribe();

        let client_id = connection.remote_id();

        tokio::spawn(send_udp_packets_to_client(connection.clone(), scanner));

        let mut local_stream = TcpStream::connect(LOCALHOST_WC3_ADDR)
            .await
            .inspect_err(|e| eprintln!("Error connecting to local TCP port: {}", e))?;

        let (mut send, mut recv) = connection
            .accept_bi()
            .await
            .inspect_err(|e| eprintln!("Error accepting incoming TCP stream: {}", e))?;

        tokio::spawn(async move {
            let mut web_connection = tokio::io::join(&mut recv, &mut send);
            if let Err(e) =
                tokio::io::copy_bidirectional(&mut web_connection, &mut local_stream).await
            {
                eprintln!(
                    "TCP port forwarding for client {} stopped with error: {}",
                    client_id, e
                );
            };
        });

        connection.closed().await;
        println!("Client disconnected: {}", connection.remote_id());

        Ok(())
    }
}

async fn send_udp_packets_to_client(
    connection: Connection,
    mut scanner: Receiver<GenerableWc3UdpMessageType>,
) {
    let mut udp_send_stream = connection
        .open_uni()
        .await
        .expect("Can't open UDP stream to client");

    loop {
        if let Ok(message) = scanner.recv().await {
            let serialized_packet =
                try_serialize(&message).expect("Failed to serialize UDP packet");

            match udp_send_stream.write_all(&serialized_packet).await {
                Err(WriteError::ConnectionLost(_)) => {
                    //Connection closed, stop sending packets
                    break;
                }
                Err(e) => {
                    eprintln!("Error sending UDP packet to client: {}", e);
                }
                Ok(_) => {}
            }
        }
    }
}
