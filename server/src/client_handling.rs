use futures_util::stream::SplitStream;
use futures_util::{future, pin_mut, SinkExt, TryStreamExt};
use serde_json::Error;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use uuid::Uuid;

use crate::api_models::ClientMessage;
use crate::{game, models};

use log::info;

pub async fn listen_for_messages(
    read_stream: SplitStream<WebSocketStream<TcpStream>>,
    addr: SocketAddr,
    lobby_id: Uuid,
    db_arc: models::DbArc,
    server_arc: models::ServerArc,
) {
    let broadcast_incoming = read_stream.try_for_each(|msg| {
        info!(
            "Received a message from {}: {}",
            addr,
            msg.to_text().unwrap()
        );

        tokio::spawn(process_message_of_client(
            lobby_id,
            addr,
            server_arc.clone(),
            db_arc.clone(),
            msg,
        ));

        future::ok(())
    });

    pin_mut!(broadcast_incoming);
    let _ = broadcast_incoming.await;

    println!("{} disconnected", addr);

    let mut server = server_arc.lock().await;
    server
        .lobbies
        .get_mut(&lobby_id)
        .unwrap()
        .clients
        .remove(&addr);

    let mut db = db_arc.lock().await;
    db.connections.remove(&addr);
}

async fn process_message_of_client(
    lobby_id: Uuid,
    addr: SocketAddr,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
    message: Message,
) {
    let mut server = server_arc.lock().await;

    if !matches!(
        server.lobbies.get(&lobby_id).unwrap().status,
        models::LobbyStatus::RUNNING
    ) {
        info!(
            "Skipping message of client with address '{}'. Lobby is not running at the moment.",
            addr
        );
        return;
    }

    if !matches!(
        server
            .lobbies
            .get(&lobby_id)
            .unwrap()
            .clients
            .get(&addr)
            .unwrap()
            .client_type,
        models::ClientType::PLAYER,
    ) {
        info!(
            "Skipping message of non PLAYER client with address '{}'.",
            addr
        );
        return;
    }

    let result: Result<ClientMessage, Error> = serde_json::from_str(&message.to_string());
    match result {
        Err(err) => {
            info!(
                "Failed to parse message from client with address '{}'. Original error: {}.",
                addr, err
            );
            return;
        }
        Ok(client_message) => {
            if server
                .lobbies
                .get_mut(&lobby_id)
                .unwrap()
                .client_messages
                .get(&addr)
                .is_some()
            {
                info!(
                    "Skipping message, because client with adddress '{}' supplied duplicate message during game tick.",
                    addr
                );
                return;
            }

            let client_tick = client_message.tick;
            let game_tick = server.lobbies.get(&lobby_id).unwrap().tick;

            if client_tick != game_tick {
                info!(
            "Skipping message, because client with adddress '{}' used invalid tick '{}'. Current tick: '{}'.",
            addr, game_tick, client_tick
        );
                return;
            }

            server
                .lobbies
                .get_mut(&lobby_id)
                .unwrap()
                .client_messages
                .insert(addr, client_message);

            tokio::spawn(game::check_all_clients_responded(
                lobby_id,
                server_arc.clone(),
                db_arc.clone(),
            ));
        }
    }
}

pub async fn send_message_to_addr(addr: &SocketAddr, message: Message, db_arc: models::DbArc) {
    let mut db = db_arc.lock().await;

    info!("Sending message to client with address '{}'", addr);

    db.connections
        .get_mut(addr)
        .expect(&format!(
            "No connection found for client with address '{}'",
            addr
        ))
        .write_stream
        .send(message)
        .await
        .expect("Sending failed");
}
