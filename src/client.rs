use std::sync::Arc;

use binrw::NullString;
use iroh::{Endpoint, EndpointAddr, endpoint::Connection};
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, UdpSocket},
};

use crate::{
    packets::Wc3UdpMessageType,
    utils::{ALPN, APP_NAME, LOCALHOST_WC3_ADDR, ZERO_SOCKET_ADDR, try_serialize},
};

pub async fn run_client(address: EndpointAddr) {
    let endpoint = Endpoint::bind().await.expect("Can't create endpoint");

    let connection = endpoint
        .connect(address, ALPN)
        .await
        .expect("Can't connect to host");

    println!("Connection established");

    let tcp_client = TcpListener::bind(ZERO_SOCKET_ADDR)
        .await
        .expect("Can't create TCP client");

    let random_port = tcp_client
        .local_addr()
        .expect("Can't determine local TCP port")
        .port();

    tokio::spawn(connect_tcp_port_to_iroh(tcp_client, connection.clone()));
    tokio::spawn(forward_udp_packets_to_game(connection.clone(), random_port));

    connection.closed().await;
    println!("The server has closed the connection");
}

async fn connect_tcp_port_to_iroh(local_socket: TcpListener, web_connection: Connection) {
    loop {
        match local_socket.accept().await {
            Ok((mut local_tcp_stream, _)) => {
                let cloned_conn = web_connection.clone();
                tokio::spawn(async move {
                    let mut web_stream = {
                        let (send_stream, recv_stream) = cloned_conn
                            .open_bi()
                            .await
                            .expect("Failed to open stream to host");
                        tokio::io::join(recv_stream, send_stream)
                    };

                    if let Err(e) = copy_bidirectional(&mut web_stream, &mut local_tcp_stream).await
                    {
                        eprintln!("TCP port forwarding stopped with error: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Can't accept TCP stream: {}", e);
            }
        }
    }
}

async fn forward_udp_packets_to_game(connection: Connection, tcp_port: u16) {
    //No loop needed, as this is a single stream per connection
    let mut udp_web_recv = connection
        .accept_uni()
        .await
        .expect("Can't accept UDP stream from host");

    let local_udp_sender = Arc::from(
        UdpSocket::bind(ZERO_SOCKET_ADDR)
            .await
            .expect("Can't create UDP sender"),
    );
    local_udp_sender
        .connect(LOCALHOST_WC3_ADDR)
        .await
        .expect("Can't connect local UDP socket to local game");

    let forward_package = async |packet: &[u8]| {
        if let Err(e) = local_udp_sender.send(packet).await {
            eprintln!("Error sending data to local game: {:?}", e);
        }
    };

    let mut server_detected = false;

    let mut buf = [0; 1024];
    while let Some(len) = udp_web_recv
        .read(&mut buf)
        .await
        .expect("Can't read from UDP web tunnel")
    {
        let data = &buf[0..len];

        match Wc3UdpMessageType::detect(data) {
            Some(Wc3UdpMessageType::QueryForGamesResponse(mut response)) => {
                if !server_detected {
                    println!(
                        "Found game on server: {} {:?}[V1.{}]",
                        response.game_name, response.game_type, response.game_version
                    );
                    server_detected = true;
                }
                response.tcp_port = tcp_port;
                let mut new_name = format!("[{}] {}", APP_NAME, response.game_name);
                new_name.truncate(31); //Trim to max 31 chars for WC3 size limit
                response.packet_size -= response.game_name.len() as u16;
                response.packet_size += new_name.len() as u16;
                response.game_name = NullString::from(new_name);
                forward_package(
                    &try_serialize(&response)
                        .expect("Failed to serialize modified QueryForGamesResponse packet"),
                )
                .await;
            }
            Some(Wc3UdpMessageType::NewServerHosted) => forward_package(data).await,
            Some(Wc3UdpMessageType::ServerCanceled) => {
                println!("Server has canceled the game or is no longer available.");
                server_detected = false;
                forward_package(data).await;
            }
            _ => {}
        };
    }
}
