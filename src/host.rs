use std::{net::SocketAddr, sync::Arc};

use iroh::{
    Endpoint,
    endpoint::Connection,
    protocol::{AcceptError, ProtocolHandler, Router},
};
use tokio::net::{TcpStream, UdpSocket};

use crate::utils::{
    ALPN_TCP_6112, ALPN_UDP_6112, LOCALHOST_V4, WC3_DEFAULT_PORT, ZERO_SOCKET_ADDR,
};

pub async fn demo_host() {
    let ep = Endpoint::builder()
        .bind()
        .await
        .expect("Can't create endpoint");

    let _router = Router::builder(ep.clone())
        .accept(ALPN_TCP_6112, TcpPortClient::new(WC3_DEFAULT_PORT))
        .accept(ALPN_UDP_6112, UdpPortClient::new(WC3_DEFAULT_PORT))
        .spawn();

    ep.online().await;
    println!("Host is running with address: {}", ep.addr().id);

    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for event");
    println!("Shutting down host...");
}

#[derive(Debug, Clone)]
struct TcpPortClient {
    port: u16,
}

impl TcpPortClient {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

impl ProtocolHandler for TcpPortClient {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        println!(
            "Received TCP stream for port {} from {}",
            self.port,
            connection.remote_id()
        );
        let mut local_stream = TcpStream::connect(SocketAddr::from((LOCALHOST_V4, self.port)))
            .await
            .expect("Can't connect to local tcp port");

        let (mut send, mut recv) = connection
            .accept_bi()
            .await
            .expect("Can't accept incoming tcp stream");
        let mut web_connection = tokio::io::join(&mut recv, &mut send);

        tokio::io::copy_bidirectional(&mut web_connection, &mut local_stream)
            .await
            .expect("Error during copy_bidirectional on host");

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct UdpPortClient {
    port: u16,
}

impl UdpPortClient {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

impl ProtocolHandler for UdpPortClient {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        println!(
            "Received UDP stream for port {} from {}",
            self.port,
            connection.remote_id()
        );

        let listen_socket = Arc::from(UdpSocket::bind(ZERO_SOCKET_ADDR).await?);
        listen_socket
            .connect(SocketAddr::from((LOCALHOST_V4, self.port)))
            .await?; //Limit socket to only communicate with local server

        let send_socket = listen_socket.clone();

        let (mut send, mut recv) = connection.accept_bi().await?;

        let web_to_client_task = tokio::task::spawn(async move {
            let mut buffer = [0u8; 1024];
            loop {
                match recv.read(&mut buffer).await {
                    Ok(Some(len)) => {
                        let data = &buffer[..len];
                        match send_socket.send(data).await {
                            Result::Ok(_) => {}
                            Result::Err(e) => {
                                println!("Error sending data: {:?}", e);
                            }
                        };
                    }
                    Ok(None) => {
                        println!("UDP stream closed by remote");
                        break;
                    }
                    Result::Err(e) => {
                        println!("Error receiving data from web: {:?}", e);
                        break;
                    }
                }
            }
        });

        let client_to_web_task = tokio::task::spawn(async move {
            let mut buffer = [0u8; 1024];
            loop {
                match listen_socket.recv(&mut buffer).await {
                    Result::Ok(len) => {
                        let data = &buffer[..len];
                        send.write_all(data)
                            .await
                            .expect("Can't write udp traffic to stream");
                    }
                    Result::Err(e) => {
                        println!("Error receiving data: {:?}", e);
                    }
                };
            }
        });

        web_to_client_task.await.expect("UDP send failed");
        client_to_web_task.await.expect("UDP receive failed");
        Ok(())
    }
}
