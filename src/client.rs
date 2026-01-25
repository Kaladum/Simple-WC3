use std::sync::Arc;

use binrw::NullString;
use iroh::{Endpoint, EndpointAddr, endpoint::Connection};
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, UdpSocket},
};

use crate::{
    handle_error_displayed,
    packets::Wc3UdpMessageType,
    utils::{ALPN, APP_NAME, LOCALHOST_WC3_ADDR, ZERO_SOCKET_ADDR, try_serialize},
};

pub async fn run_client(address: EndpointAddr) {
    let endpoint = handle_error_displayed!(Endpoint::bind().await, "Can't create endpoint: {}");

    let connection = handle_error_displayed!(
        endpoint.connect(address, ALPN).await,
        "Can't connect to host: {}"
    );

    println!("Connection established");

    let tcp_client = handle_error_displayed!(
        TcpListener::bind(ZERO_SOCKET_ADDR).await,
        "Can't create TCP client: {}"
    );

    let random_port = handle_error_displayed!(
        tcp_client.local_addr(),
        "Can't determine local TCP port: {}"
    )
    .port();

    tokio::spawn(connect_tcp_port_to_iroh(tcp_client, connection.clone()));
    if let Err(_) = start_forwarding_udp_packets_to_game(connection.clone(), random_port).await {
        return;
    }

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
                        let (send_stream, recv_stream) = handle_error_displayed!(
                            cloned_conn.open_bi().await,
                            "Failed to open stream to host: {}"
                        );
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

async fn start_forwarding_udp_packets_to_game(
    connection: Connection,
    tcp_port: u16,
) -> Result<(), ()> {
    //No loop needed, as this is a single stream per connection
    let mut udp_web_recv = connection
        .accept_uni()
        .await
        .map_err(|e| eprintln!("Can't accept UDP stream from host: {}", e))?;

    let local_udp_sender = Arc::from(
        UdpSocket::bind(ZERO_SOCKET_ADDR)
            .await
            .map_err(|e| eprintln!("Can't create UDP sender: {}", e))?,
    );
    local_udp_sender
        .connect(LOCALHOST_WC3_ADDR)
        .await
        .map_err(|e| eprintln!("Can't connect local UDP socket to local game: {}", e))?;

    tokio::spawn(async move {
        let forward_package = async |packet: &[u8]| {
            let _ = local_udp_sender.send(packet).await; //Ignore errors, as the game might not be running and the error behavior is unpredictable
        };

        let mut server_detected = false;

        let mut handle_packet = async |data: &[u8]| {
            match Wc3UdpMessageType::detect(data) {
                Some(Wc3UdpMessageType::QueryForGamesResponse(mut response)) => {
                    if !server_detected {
                        println!(
                            "Found game on host: {} {:?}[V1.{}]",
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
                    if let Some(serialized) = &try_serialize(&response) {
                        forward_package(serialized).await;
                    } else {
                        eprintln!("Failed to serialize modified QueryForGamesResponse packet");
                    }
                }
                Some(Wc3UdpMessageType::NewServerHosted) => forward_package(data).await,
                Some(Wc3UdpMessageType::ServerCanceled) => {
                    println!(
                        "The lobby is no longer available. The game was started or canceled by the host."
                    );
                    server_detected = false;
                    forward_package(data).await;
                }
                _ => {}
            };
        };

        let mut buf = [0; 1024];
        loop {
            match udp_web_recv.read(&mut buf).await {
                Ok(Some(len)) => {
                    let data = &buf[0..len];
                    handle_packet(data).await;
                }
                Ok(None) => {
                    //Stream closed
                    break;
                }
                Err(e) => {
                    eprintln!("Can't read from UDP web tunnel: {}", e);
                    break;
                }
            }
        }
    });
    Ok(())
}
