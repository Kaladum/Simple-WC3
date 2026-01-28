use iroh::{
    Endpoint, PublicKey,
    endpoint::{Connection, RecvStream, SendStream, WriteError},
    protocol::{AcceptError, ProtocolHandler, Router},
};
use tokio::{
    net::TcpStream,
    sync::broadcast::{Receiver, Sender},
};

use crate::{
    game_scanner, handle_error_displayed,
    packets::GenerableWc3UdpMessageType,
    utils::{ALPN, LOCALHOST_WC3_ADDR, try_serialize},
};

pub async fn run_host() {
    let ep = handle_error_displayed!(
        Endpoint::builder().bind().await,
        "Can't create endpoint: {}"
    );

    let game_scanner_tx = handle_error_displayed!(
        game_scanner::run_game_scanner().await,
        "Can't start game scanner: {}"
    );

    let handler = ClientHandler {
        scanner: game_scanner_tx,
    };
    //Do not drop the router. It runs the protocol handler in the background.
    let _router = Router::builder(ep.clone()).accept(ALPN, handler).spawn();
    ep.online().await;
    println!("Host is running with address:");
    println!("{}", ep.addr().id);
    println!();
    println!(
        "Copy this address (by selecting it and right-clicking) and share it with all players to let them connect"
    );
    println!("Press Ctrl+C or close the window to shut down");
    println!();

    handle_error_displayed!(
        tokio::signal::ctrl_c().await,
        "failed to listen for event: {}"
    );
    println!("Shutting down host...");
}

#[derive(Debug, Clone)]
struct ClientHandler {
    pub scanner: Sender<GenerableWc3UdpMessageType>,
}

impl ProtocolHandler for ClientHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let client_id = connection.remote_id();
        println!("New client connected: {client_id}");

        let scanner = self.scanner.subscribe();
        tokio::spawn(send_udp_packets_to_client(connection.clone(), scanner));
        tokio::spawn(accept_tcp_forwarding(connection.clone()));

        connection.closed().await;
        println!("Client disconnected: {client_id}");

        Ok(())
    }
}

async fn send_udp_packets_to_client(
    connection: Connection,
    mut scanner: Receiver<GenerableWc3UdpMessageType>,
) {
    let mut udp_send_stream = handle_error_displayed!(
        connection.open_uni().await,
        "Can't open UDP stream to client: {}"
    );

    loop {
        if let Ok(message) = scanner.recv().await {
            let serialized_packet = if let Some(serialized_packet) = try_serialize(&message) {
                serialized_packet
            } else {
                eprintln!("Failed to serialize UDP packet");
                continue;
            };

            match udp_send_stream.write_all(&serialized_packet).await {
                Err(
                    WriteError::ConnectionLost(_)
                    | WriteError::ClosedStream
                    | WriteError::Stopped(_),
                ) => {
                    //Connection or stream closed, stop sending packets
                    break;
                }
                Err(e) => {
                    eprintln!("Error sending UDP packet to client: {e}");
                }
                Ok(_) => {}
            }
        }
    }
}

async fn accept_tcp_forwarding(connection: Connection) {
    let client_id = connection.remote_id();

    loop {
        match connection.accept_bi().await {
            Ok((send, recv)) => {
                tokio::spawn(async move {
                    let _ = handle_tcp_forwarding_connection(send, recv, client_id).await;
                });
            }
            Err(e) => {
                if connection.close_reason().is_some() {
                    break;
                } else {
                    eprintln!("Error accepting incoming TCP stream from client {client_id}: {e}");
                }
            }
        };
    }
}

async fn handle_tcp_forwarding_connection(
    mut send: SendStream,
    mut recv: RecvStream,
    client_id: PublicKey,
) -> Result<(), ()> {
    let mut local_stream = TcpStream::connect(LOCALHOST_WC3_ADDR)
        .await
        .map_err(|e| eprintln!("Error connecting to local TCP port for client {client_id}: {e}"))?;

    let mut web_connection = tokio::io::join(&mut recv, &mut send);
    tokio::io::copy_bidirectional(&mut web_connection, &mut local_stream)
        .await
        .map_err(|e| {
            eprintln!("TCP port forwarding for client {client_id} stopped with error: {e}")
        })?;
    Ok(())
}
