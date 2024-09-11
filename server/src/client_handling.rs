use futures_util::stream::SplitStream;
use futures_util::{future, pin_mut, SinkExt, TryStreamExt};
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use uuid::Uuid;

use crate::models;

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
