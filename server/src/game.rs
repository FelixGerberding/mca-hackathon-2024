use log::info;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

use crate::api_models;
use crate::client_handling;
use crate::models;
use crate::models::GameState;
use crate::models::Player;

pub async fn start_game_for_lobby(
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) {
    let mut server = server_arc.lock().await;

    let lobby = server.lobbies.get_mut(&lobby_id).unwrap();

    lobby.game_state = get_initial_game_state(lobby);

    tokio::spawn(run_game_for_lobby(
        lobby_id,
        server_arc.clone(),
        db_arc.clone(),
    ));
}

fn get_initial_game_state(lobby: &mut models::Lobby) -> Option<GameState> {
    let players = lobby
        .clients
        .iter()
        .zip(0i32..)
        .filter_map(|((addr, client), index)| match client.client_type {
            models::ClientType::PLAYER => {
                let player = models::Player {
                    entity_type: models::EntityType::PLAYER,
                    id: Uuid::new_v4(),
                    name: client.username.clone(),
                    x: index,
                    y: index,
                    rotation: 100,
                    color: "#FF00FF".to_string(),
                    health: 100,
                    last_action_success: true,
                    error_message: "".to_string(),
                };

                return Some((addr.clone(), player));
            }
            _ => None,
        })
        .collect();

    let game_state = GameState {
        players: players,
        entities: Vec::new(),
    };

    return Some(game_state);
}

async fn run_game_for_lobby(lobby_id: Uuid, server_arc: models::ServerArc, db_arc: models::DbArc) {
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

    let lobby = server.lobbies.get_mut(&lobby_id).unwrap();

    let socket_addresses: Vec<SocketAddr> = lobby.clients.keys().cloned().collect();

    let game_state = lobby.game_state.clone().unwrap();
    let game_state_out = api_models::GameStateOut {
        entities: game_state.entities,
        players: transform_map_of_players_to_list_of_player(game_state.players),
    };

    for addr in socket_addresses {
        let game_state_string = serde_json::to_string(&game_state_out).unwrap();
        client_handling::send_message_to_addr(
            &addr,
            tokio_tungstenite::tungstenite::Message::Text(game_state_string),
            db_arc.clone(),
        )
        .await;
    }
}

fn transform_map_of_players_to_list_of_player(
    map_of_player: HashMap<SocketAddr, Player>,
) -> Vec<Player> {
    return map_of_player.values().cloned().collect();
}

pub fn update_state_of_player(
    client_message: api_models::ClientMessage,
    player: &mut models::Player,
) {
    match client_message.action {
        api_models::ClientAction::TURN => {}
        api_models::ClientAction::UP => {
            player.y += 1;
        }
        api_models::ClientAction::DOWN => {
            player.y -= 1;
        }
        api_models::ClientAction::RIGHT => {
            player.x += 1;
        }
        api_models::ClientAction::LEFT => {
            player.x -= 1;
        }
    }
}
