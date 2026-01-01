use std::sync::Arc;

use binrw::NullString;
use iroh::{Endpoint, EndpointAddr, endpoint::Connection, protocol::AcceptError};
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, UdpSocket},
};

use crate::{
    packets::{GameType, QueryForGamesRequest, Wc3UdpMessageType},
    utils::{
        ALPN_TCP_6112, ALPN_UDP_6112, APP_NAME, LOCALHOST_WC3_ADDR, ZERO_SOCKET_ADDR, try_serialize,
    },
};

pub async fn run_client(address: EndpointAddr) {
    let endpoint = Endpoint::bind().await.expect("Can't create endpoint");

    let tcp_connection = endpoint
        .connect(address.clone(), ALPN_TCP_6112)
        .await
        .expect("Can't connect TCP tunnel to host");

    let udp_connection = endpoint
        .connect(address, ALPN_UDP_6112)
        .await
        .expect("Can't connect UDP tunnel to host");

    let (mut udp_web_send, mut udp_web_recv) = udp_connection
        .open_bi()
        .await
        .expect("Can't open UDP stream to host");

    let tcp_client = TcpListener::bind(ZERO_SOCKET_ADDR)
        .await
        .expect("Can't create TCP client");

    let random_port = tcp_client
        .local_addr()
        .expect("Can't determine local TCP port")
        .port();

    tokio::spawn(connect_tcp_port_to_iroh(tcp_client, tcp_connection));

    let local_udp_sender = Arc::from(
        UdpSocket::bind(ZERO_SOCKET_ADDR)
            .await
            .expect("Can't create UDP sender"),
    );
    local_udp_sender
        .connect(LOCALHOST_WC3_ADDR)
        .await
        .expect("Can't connect local UDP socket to local game");

    tokio::spawn(async move {
        let query_for_game =
            try_serialize(&QueryForGamesRequest::new(GameType::TheFrozenThrone, 26))
                .expect("Failed to serialize QueryForGamesRequest packet");
        loop {
            if let Err(e) = udp_web_send.write_all(&query_for_game).await {
                eprintln!("Can't send game query to host: {}", e);
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });

    let mut buf = [0; 1024];
    while let Some(len) = udp_web_recv
        .read(&mut buf)
        .await
        .expect("Can't read from UDP web tunnel")
    {
        let data = &buf[0..len];

        let forward = match Wc3UdpMessageType::detect(data) {
            Some(Wc3UdpMessageType::QueryForGamesResponse(mut response)) => {
                println!("Modifying QueryForGamesResponse packet");
                response.tcp_port = random_port;
                let mut new_name = format!("[{}] {}", APP_NAME, response.game_name);
                new_name.truncate(31); //Trim to max 31 chars for WC3 size limit
                response.packet_size -= response.game_name.len() as u16;
                response.packet_size += new_name.len() as u16;
                response.game_name = NullString::from(new_name);
                Some(
                    try_serialize(&response)
                        .expect("Failed to serialize modified QueryForGamesResponse packet"),
                )
            }
            Some(packet_type) => {
                println!("Forwarding packet type: {:?}", packet_type);
                Some(data.to_vec())
            }
            _ => None,
        };

        if let Some(buffer) = forward {
            if let Err(e) = local_udp_sender.send(&buffer).await {
                eprintln!("Error sending data to local game: {:?}", e);
            }
        }
    }
}

async fn connect_tcp_port_to_iroh(
    local_socket: TcpListener,
    web_connection: Connection,
) -> Result<(), AcceptError> {
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
                        eprintln!("Error during copy_bidirectional on client: {}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Can't accept TCP stream: {}", e);
            }
        }
    }
}
