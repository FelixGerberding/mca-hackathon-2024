use log::info;
use std::collections::HashMap;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time;
use uuid::Uuid;

use crate::api_models;
use crate::client_handling;
use crate::models;
use crate::models::GameState;
use crate::models::Player;

const MAX_FIELD_SIZE_X: i32 = 30;
const MAX_FIELD_SIZE_Y: i32 = 30;
const MAX_ROUNDS: i32 = 10;

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
                    x: index + 5,
                    y: index + 5,
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
    info!("Starting game for lobby with id '{}'.", lobby_id);

    let server = server_arc.lock().await;

    schedule_next_client_update(
        server.lobbies.get(&lobby_id).unwrap().tick,
        lobby_id,
        server_arc.clone(),
        db_arc.clone(),
        false,
    )
    .await;
}

async fn ping_clients_in_lobby(
    expected_tick: Uuid,
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
    should_wait: bool,
) {
    if should_wait {
        time::sleep(Duration::from_secs(2)).await;
    }

    info!("Running update of game state");
    let mut server = server_arc.lock().await;

    let lobby = server.lobbies.get_mut(&lobby_id).unwrap();
    if expected_tick != lobby.tick {
        info!(
            "Skipping scheduled update of clients for tick '{}'",
            expected_tick
        );
        return;
    }

    lobby
        .client_messages
        .iter()
        .for_each(|(addr, client_message)| {
            update_state_of_player(
                client_message.clone(),
                lobby
                    .game_state
                    .as_mut()
                    .unwrap()
                    .players
                    .get_mut(addr)
                    .unwrap(),
            );
        });

    lobby.tick = Uuid::new_v4();
    lobby.round += 1;

    let socket_addresses: Vec<SocketAddr> = lobby.clients.keys().cloned().collect();

    let game_state = lobby.game_state.clone().unwrap();
    let game_state_out = api_models::GameStateOut {
        tick: lobby.tick,
        tick_length_milli_seconds: lobby.tick_length_milli_seconds,
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

    if lobby.round >= MAX_ROUNDS {
        info!(
            "Maximum of rounds ({}) was reached, stopping lobby.",
            MAX_ROUNDS
        );
        lobby.status = models::LobbyStatus::FINISHED;
        return;
    }

    lobby.client_messages = HashMap::new();

    schedule_next_client_update(
        lobby.tick,
        lobby_id,
        server_arc.clone(),
        db_arc.clone(),
        true,
    )
    .await;
}

async fn schedule_next_client_update(
    expected_tick: Uuid,
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
    should_wait: bool,
) {
    let mut db = db_arc.lock().await;

    let handle = start_next_client_update(
        expected_tick,
        lobby_id,
        server_arc.clone(),
        db_arc.clone(),
        should_wait,
    );

    db.open_tick_handles.insert(expected_tick, handle);
}

fn start_next_client_update(
    expected_tick: Uuid,
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
    should_wait: bool,
) -> JoinHandle<()> {
    tokio::spawn(ping_clients_in_lobby(
        expected_tick,
        lobby_id,
        server_arc.clone(),
        db_arc.clone(),
        should_wait,
    ))
}

fn transform_map_of_players_to_list_of_player(
    map_of_player: HashMap<SocketAddr, Player>,
) -> Vec<Player> {
    return map_of_player.values().cloned().collect();
}

fn update_state_of_player(client_message: api_models::ClientMessage, player: &mut models::Player) {
    player.error_message = "".to_string();
    player.last_action_success = true;

    match client_message.action {
        api_models::ClientAction::TURN => {
            if client_message.degrees.is_none() {
                player.error_message =
                    "Cannot TURN, because no 'degrees' property was supplied".to_string();
                player.last_action_success = false;
                return;
            }

            let degrees = client_message.degrees.unwrap();

            if !(degrees >= 0 && degrees <= 360) {
                player.error_message =
                    "Cannot TURN, because 'degrees' is not within range (0 - 360)".to_string();
                player.last_action_success = false;
                return;
            }

            player.rotation = degrees;
        }
        api_models::ClientAction::UP => {
            if player.y < MAX_FIELD_SIZE_Y {
                player.y += 1;
            } else {
                player.error_message =
                    "Cannot move UP, because player is at border of field".to_string();
                player.last_action_success = false;
            }
        }
        api_models::ClientAction::DOWN => {
            if player.y > 0 {
                player.y -= 1;
            } else {
                player.error_message =
                    "Cannot move DOWN, because player is at border of field".to_string();
                player.last_action_success = false;
            }
        }
        api_models::ClientAction::RIGHT => {
            if player.x < MAX_FIELD_SIZE_X {
                player.x += 1;
            } else {
                player.error_message =
                    "Cannot move RIGHT, because player is at border of field".to_string();
                player.last_action_success = false;
            }
        }
        api_models::ClientAction::LEFT => {
            if player.x > 0 {
                player.x -= 1;
            } else {
                player.error_message =
                    "Cannot move LEFT, because player is at border of field".to_string();
                player.last_action_success = false;
            }
        }
    }
}

pub async fn check_all_clients_responded(
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) {
    info!("Checking if all clients of lobby have responded");
    let server = server_arc.lock().await;

    let expected_client_addresses: HashSet<SocketAddr> = server
        .lobbies
        .get(&lobby_id)
        .unwrap()
        .clients
        .iter()
        .filter_map(|(addr, client)| {
            if matches!(client.client_type, models::ClientType::PLAYER) {
                return Some(addr);
            }

            None
        })
        .cloned()
        .collect();

    let clients_that_answered: HashSet<SocketAddr> = server
        .lobbies
        .get(&lobby_id)
        .unwrap()
        .client_messages
        .keys()
        .cloned()
        .collect();

    if expected_client_addresses == clients_that_answered {
        let mut db = db_arc.lock().await;

        let expected_tick = server.lobbies.get(&lobby_id).unwrap().tick;

        info!(
            "Triggering premature lobby update, because all clients responded for tick '{}'",
            expected_tick
        );

        db.open_tick_handles.get(&expected_tick).unwrap().abort();
        db.open_tick_handles.remove(&expected_tick);

        info!("Canceled scheduled update for tick '{}'", expected_tick);

        tokio::spawn(ping_clients_in_lobby(
            expected_tick,
            lobby_id,
            server_arc.clone(),
            db_arc.clone(),
            false,
        ));
    }
}
