use futures_util::stream::SplitStream;
use futures_util::{future, pin_mut, SinkExt, TryStreamExt};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use uuid::Uuid;

use crate::models;
use crate::{api_models, game};

use log::info;

pub async fn listen_for_messages(
    read_stream: SplitStream<WebSocketStream<TcpStream>>,
    addr: SocketAddr,
    lobby_id: Uuid,
    db_arc: models::DbArc,
    server_arc: models::ServerArc,
) {
    let broadcast_incoming = read_stream.try_for_each(|msg| {
        println!(
            "Received a message from {}: {}",
            addr,
            msg.to_text().unwrap()
        );

        tokio::spawn(process_message_of_client(
            lobby_id,
            addr,
            server_arc.clone(),
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

    let client_message: api_models::ClientMessage = serde_json::from_str(&message.to_string())
        .expect(&format!("Cannot parse message of client {}", addr));

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

    server
        .lobbies
        .get_mut(&lobby_id)
        .unwrap()
        .client_messages
        .insert(addr, client_message);
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
