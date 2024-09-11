use futures_util::{stream, StreamExt};
use log::info;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

use crate::client_handling;
use crate::models;

pub async fn start_game_for_lobby(
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) {
    info!("Starting game for lobby with id '{}'", lobby_id);

    for _i in 0..5 {
        time::sleep(Duration::from_secs(2)).await;
        let _ = tokio::spawn(ping_clients_in_lobby(
            lobby_id.clone(),
            server_arc.clone(),
            db_arc.clone(),
        ))
        .await;
    }

    let mut server = server_arc.lock().await;

    let lobby = server.lobbies.get_mut(&lobby_id).unwrap();

    lobby.status = models::LobbyStatus::FINISHED;
}

async fn ping_clients_in_lobby(
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) {
    let mut server = server_arc.lock().await;

    let lobby = server.lobbies.get(&lobby_id).unwrap();

    let socket_addresses: Vec<SocketAddr> = lobby.clients.keys().cloned().collect();

    for addr in socket_addresses {
        client_handling::send_message_to_addr(
            &addr,
            tokio_tungstenite::tungstenite::Message::Text("Test".to_string()),
            db_arc.clone(),
        )
        .await;
    }
}
