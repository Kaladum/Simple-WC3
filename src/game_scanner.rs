use std::{sync::Arc, time::Duration};
use tokio::{
    net::UdpSocket,
    sync::{
        Mutex,
        broadcast::{self, Sender},
    },
};

use crate::{
    packets::{
        GenerableWc3UdpMessageType, NewServerHosted, QueryForGamesRequest, QueryForGamesResponse,
        ServerClosed, Wc3UdpMessageType,
    },
    utils::{
        LOCALHOST_WC3_ADDR, SUPPORTED_GAME_TYPES, SUPPORTED_GAME_VERSIONS, ZERO_SOCKET_ADDR,
        try_serialize,
    },
};

pub async fn run_game_scanner() -> Sender<GenerableWc3UdpMessageType> {
    let listen_socket: Arc<_> = UdpSocket::bind(ZERO_SOCKET_ADDR)
        .await
        .expect("Error binding local UDP socket")
        .into();
    listen_socket
        .connect(LOCALHOST_WC3_ADDR)
        .await
        .expect("Binding UDP port to localhost failed"); //Limit socket to only communicate with local server

    let send_socket = listen_socket.clone();

    let (tx, _rx) = broadcast::channel::<GenerableWc3UdpMessageType>(1);
    let tx_external = tx.clone();

    let last_known_state = Arc::new(Mutex::new(Option::<QueryForGamesResponse>::None));
    let last_known_state_set = last_known_state.clone();

    tokio::spawn(async move {
        let broadcast_packet = |packet: GenerableWc3UdpMessageType| {
            //This error can be ignored, it only happens if there are no listeners
            let _ = tx.send(packet);
        };

        let mut last_send_successful = Option::<bool>::None;

        loop {
            let old_state = {
                // Clear last known state before sending new queries
                let mut state = last_known_state.lock().await;
                state.take()
            };
            send_game_query(&send_socket, &mut last_send_successful).await;
            tokio::time::sleep(Duration::from_secs(1)).await;
            let new_state = {
                let state = last_known_state.lock().await;
                state.clone()
            };
            match (old_state, new_state) {
                (None, Some(state)) => {
                    println!(
                        "Discovered new game server: {} {:?}[1.{}]",
                        state.game_name, state.game_type, state.game_version
                    );
                    broadcast_packet(GenerableWc3UdpMessageType::NewServerHosted(
                        NewServerHosted {
                            game_id: state.game_id,
                            game_type: state.game_type,
                            game_version: state.game_version,
                        },
                    ));
                    broadcast_packet(GenerableWc3UdpMessageType::QueryForGamesResponse(state));
                }
                (Some(_), Some(state)) => {
                    broadcast_packet(GenerableWc3UdpMessageType::QueryForGamesResponse(state));
                }
                (Some(old_state), None) => {
                    println!(
                        "Server closed: {} {:?}[1.{}]",
                        old_state.game_name, old_state.game_type, old_state.game_version
                    );
                    broadcast_packet(GenerableWc3UdpMessageType::ServerClosed(ServerClosed {
                        game_id: old_state.game_id,
                    }));
                }
                (None, None) => {}
            }
        }
    });

    tokio::spawn(async move { run_port_listener(listen_socket, last_known_state_set).await });

    tx_external
}

async fn run_port_listener(
    listen_socket: Arc<UdpSocket>,
    last_known_state_set: Arc<Mutex<Option<QueryForGamesResponse>>>,
) -> ! {
    let mut buffer = [0u8; 1024];
    loop {
        if let Result::Ok(len) = listen_socket.recv(&mut buffer).await {
            let data = &buffer[..len];

            match Wc3UdpMessageType::detect(data) {
                Some(Wc3UdpMessageType::QueryForGamesResponse(response)) => {
                    let mut state = last_known_state_set.lock().await;
                    *state = Some(response.clone());
                }
                Some(packet) => eprintln!("Received UDP packet: {:?}", packet),
                None => eprintln!("Received unknown UDP packet of length {}", len),
            };
        };
    }
}

async fn send_game_query(send_socket: &UdpSocket, last_successful: &mut Option<bool>) {
    for game_version in SUPPORTED_GAME_VERSIONS {
        for game_type in SUPPORTED_GAME_TYPES {
            let request = QueryForGamesRequest::new(game_type, game_version);
            let bytes =
                try_serialize(&request).expect("Failed to serialize QueryForGamesRequest packet");

            match send_socket.send(&bytes).await {
                Ok(_) => {
                    if *last_successful != Some(true) {
                        println!("Successfully sent game query to WC3");
                    }
                    *last_successful = Some(true);
                }
                Err(e) => {
                    if *last_successful != Some(false) {
                        eprintln!(
                            "Can't send game query to WC3. Is the game running? Error: {}",
                            e
                        );
                    }
                    *last_successful = Some(false);
                }
            }
        }
    }
}
