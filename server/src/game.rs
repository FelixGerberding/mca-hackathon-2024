use log::info;
use std::collections::HashMap;
use std::collections::HashSet;
use std::f64::consts::PI;
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
const MAX_ROUNDS: i32 = 500;
const PROJECTILE_UNIT_LENGTH_TRAVEL_DISTANCE: f64 = 6.0;

pub async fn start_game_for_lobby(
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) {
    let mut server = server_arc.lock().await;

    let lobby = server.lobbies.get_mut(&lobby_id).unwrap();

    lobby.client_messages = HashMap::new();
    lobby.round = 0;

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

    tokio::spawn(ping_clients_in_lobby(
        server.lobbies.get(&lobby_id).unwrap().tick,
        lobby_id,
        server_arc.clone(),
        db_arc.clone(),
    ));
}

async fn ping_clients_in_lobby(
    expected_tick: Uuid,
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) {
    info!("Running update of game state");
    let mut server = server_arc.lock().await;

    let lobby = server.lobbies.get_mut(&lobby_id).unwrap();
    if expected_tick != lobby.tick {
        info!(
            "Skipping scheduled update of clients for tick '{}'",
            expected_tick
        );

        schedule_next_client_update(lobby.tick, lobby_id, server_arc.clone(), db_arc.clone()).await;

        return;
    }

    let game_state = lobby.game_state.as_mut().unwrap();

    lobby
        .client_messages
        .iter()
        .for_each(|(addr, client_message)| {
            handle_client_message(client_message.clone(), addr, game_state);
        });

    calculate_projectile_updates(game_state);

    push_game_state_to_clients_in_lobby(lobby, db_arc.clone()).await;

    schedule_next_client_update(lobby.tick, lobby_id, server_arc.clone(), db_arc.clone()).await;
}

async fn push_game_state_to_clients_in_lobby(lobby: &mut models::Lobby, db_arc: models::DbArc) {
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
        tokio::spawn(client_handling::send_message_to_addr(
            addr.clone(),
            tokio_tungstenite::tungstenite::Message::Text(game_state_string),
            db_arc.clone(),
        ));
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
}

async fn schedule_next_client_update(
    expected_tick: Uuid,
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) {
    let mut db = db_arc.lock().await;

    let handle = schedule_deffered_client_update(
        expected_tick,
        lobby_id,
        server_arc.clone(),
        db_arc.clone(),
    );

    db.open_tick_handles.insert(expected_tick, handle);
}

fn schedule_deffered_client_update(
    expected_tick: Uuid,
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) -> JoinHandle<()> {
    tokio::spawn(run_deffered_client_update(
        expected_tick,
        lobby_id,
        server_arc.clone(),
        db_arc.clone(),
    ))
}

async fn run_deffered_client_update(
    expected_tick: Uuid,
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) {
    time::sleep(Duration::from_secs(2)).await;
    ping_clients_in_lobby(expected_tick, lobby_id, server_arc.clone(), db_arc.clone()).await;
}

fn transform_map_of_players_to_list_of_player(
    map_of_player: HashMap<SocketAddr, Player>,
) -> Vec<Player> {
    return map_of_player.values().cloned().collect();
}

pub async fn handle_client_connect(
    lobby: &mut models::Lobby,
    addr: SocketAddr,
    new_client: models::Client,
    db_arc: models::DbArc,
) {
    lobby.clients.insert(addr, new_client);

    lobby.game_state = get_initial_game_state(lobby);

    push_game_state_to_clients_in_lobby(lobby, db_arc.clone()).await;
}

pub async fn handle_client_disconnect(
    lobby: &mut models::Lobby,
    addr: SocketAddr,
    db_arc: models::DbArc,
) {
    lobby.clients.remove(&addr);

    lobby.game_state.as_mut().unwrap().players.remove(&addr);

    push_game_state_to_clients_in_lobby(lobby, db_arc.clone()).await;
}

fn handle_client_message(
    client_message: api_models::ClientMessage,
    addr: &SocketAddr,
    game_state: &mut models::GameState,
) {
    let player = game_state.players.get_mut(addr).unwrap();

    player.error_message = "".to_string();
    player.last_action_success = true;

    match client_message.action {
        api_models::ClientAction::SHOOT => {
            let new_projectile = models::Projectile {
                travel_distance: PROJECTILE_UNIT_LENGTH_TRAVEL_DISTANCE,
                id: Uuid::new_v4(),
                previous_x: player.x.into(),
                previous_y: player.y.into(),
                x: player.x.into(),
                y: player.y.into(),
                direction: player.rotation,
                source: addr.clone(),
            };

            game_state.entities.push(new_projectile);
        }
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
            if player.y < MAX_FIELD_SIZE_Y - 1 {
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
            if player.x < MAX_FIELD_SIZE_X - 1 {
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
        ));
    }
}

fn calculate_projectile_updates(game_state: &mut models::GameState) {
    game_state.entities.iter_mut().for_each(|projectile| {
        let list_of_hit_coordinates = get_fields_passed_by_projectile(projectile);

        game_state.players.iter_mut().for_each(|(addr, player)| {
            if list_of_hit_coordinates.contains(&(player.x, player.y))
                && projectile.source != addr.clone()
            {
                player.health -= 20;
                return;
            }
        });

        let ending_coordinates =
            get_ending_coordinates_of_projectile(projectile.x, projectile.y, projectile.direction);

        projectile.previous_x = projectile.x;
        projectile.previous_y = projectile.y;
        projectile.x = ending_coordinates.0;
        projectile.y = ending_coordinates.1;
    })
}

fn get_fields_passed_by_projectile(projectile: &models::Projectile) -> Vec<(i32, i32)> {
    let start_point: line_drawing::Point<f64> = (projectile.x, projectile.y);

    let end_point: line_drawing::Point<f64> =
        get_ending_coordinates_of_projectile(projectile.x, projectile.y, projectile.direction);
    return line_drawing::Midpoint::new(start_point, end_point).collect();
}

fn get_ending_coordinates_of_projectile(start_x: f64, start_y: f64, direction: i32) -> (f64, f64) {
    let directional_vector = get_directional_vector_from_degrees(direction);

    let end_x = start_x + PROJECTILE_UNIT_LENGTH_TRAVEL_DISTANCE * directional_vector.0;
    let end_y = start_y + PROJECTILE_UNIT_LENGTH_TRAVEL_DISTANCE * directional_vector.1;

    return (end_x, end_y);
}

fn get_directional_vector_from_degrees(degrees: i32) -> (f64, f64) {
    let degrees_f64: f64 = degrees.into();

    let radians = ((90.0 - degrees_f64) * PI) / 180.0;

    return (f64::cos(radians), f64::sin(radians));
}
