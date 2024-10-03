use lazy_static::lazy_static;
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
use crate::models::Player;

const MAX_FIELD_SIZE_X: i32 = 30;
const MAX_FIELD_SIZE_Y: i32 = 30;
const MAX_ROUNDS: i32 = 500;
const PROJECTILE_UNIT_LENGTH_TRAVEL_DISTANCE: f64 = 6.0;

lazy_static! {
    static ref PLAYER_COUNT_TO_POSITIONS: HashMap<usize, Vec<(i32, i32, i32)>> = {
        let mut m = HashMap::new();
        m.insert(0, vec![]);
        m.insert(1, vec![(14, 14, 0)]);
        m.insert(2, vec![(5, 14, 270), (24, 14, 90)]);
        m.insert(3, vec![(5, 5, 270), (24, 5, 90), (14, 24, 0)]);
        m.insert(
            4,
            vec![(5, 5, 270), (24, 5, 90), (5, 24, 270), (24, 24, 90)],
        );
        m.insert(
            5,
            vec![
                (5, 5, 270),
                (24, 5, 90),
                (5, 24, 270),
                (24, 24, 90),
                (14, 14, 0),
            ],
        );
        m.insert(
            6,
            vec![
                (5, 5, 270),
                (5, 14, 270),
                (5, 24, 270),
                (24, 5, 90),
                (24, 14, 90),
                (24, 24, 90),
            ],
        );
        m.insert(
            7,
            vec![
                (5, 5, 270),
                (5, 14, 270),
                (5, 24, 270),
                (24, 5, 90),
                (24, 14, 90),
                (24, 24, 90),
                (14, 14, 0),
            ],
        );
        m
    };
    static ref PLAYER_COUNT_TO_COLOR: HashMap<usize, String> = {
        let mut m = HashMap::new();
        m.insert(1, "#FF0000".to_string());
        m.insert(2, "#00FF00".to_string());
        m.insert(3, "#0000FF".to_string());
        m.insert(4, "#C800FF".to_string());
        m.insert(5, "#00FFE1".to_string());
        m.insert(6, "#FF9D00".to_string());
        m.insert(7, "#0F754C".to_string());
        m
    };
}

pub async fn start_game_for_lobby(
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) {
    let mut server = server_arc.lock().await;

    let lobby = server.lobbies.get_mut(&lobby_id).unwrap();

    lobby.client_messages = HashMap::new();
    lobby.round = 0;

    let _ = update_initial_player_positions(lobby);
    reset_player_health(lobby);

    tokio::spawn(run_game_for_lobby(
        lobby_id,
        server_arc.clone(),
        db_arc.clone(),
    ));
}

fn reset_player_health(lobby: &mut models::Lobby) {
    lobby.game_state.players.values_mut().for_each(|player| {
        player.health = 100;
    });
}

fn update_initial_player_positions(lobby: &mut models::Lobby) -> Result<(), String> {
    let player_count = lobby.game_state.players.len();
    if player_count > PLAYER_COUNT_TO_POSITIONS.len() {
        return Err(
            "Cannot add player, because no starting formation is maintained for the player count."
                .to_string(),
        );
    }

    let starting_positions = PLAYER_COUNT_TO_POSITIONS.get(&player_count).unwrap();

    lobby
        .game_state
        .players
        .values_mut()
        .enumerate()
        .for_each(|(index, player)| {
            player.x = starting_positions[index].0;
            player.y = starting_positions[index].1;
            player.rotation = starting_positions[index].2;
        });

    return Ok(());
}

async fn run_game_for_lobby(lobby_id: Uuid, server_arc: models::ServerArc, db_arc: models::DbArc) {
    info!("Starting game for lobby with id '{}'.", lobby_id);

    let mut server = server_arc.lock().await;

    let lobby = server.lobbies.get_mut(&lobby_id).unwrap();

    ping_clients_in_lobby(
        lobby.tick,
        lobby_id,
        server_arc.clone(),
        db_arc.clone(),
        lobby,
    )
    .await;
}

async fn ping_clients_in_lobby(
    expected_tick: Uuid,
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
    lobby: &mut models::Lobby,
) {
    info!("Running update of game state");

    if expected_tick != lobby.tick {
        info!(
            "Skipping scheduled update of clients for tick '{}'",
            expected_tick
        );

        return;
    }

    if lobby.status != models::LobbyStatus::RUNNING {
        info!("Skipping game update, because lobby is no longer running");
        return;
    }

    let game_state = &mut lobby.game_state;

    lobby
        .client_messages
        .iter()
        .for_each(|(addr, client_message)| {
            handle_client_message(client_message.clone(), addr, game_state);
        });

    calculate_projectile_updates(game_state);

    ping_clients_with_new_tick(lobby, db_arc.clone());

    if lobby.status == models::LobbyStatus::RUNNING {
        tokio::spawn(schedule_next_client_update(
            lobby.tick,
            lobby_id,
            server_arc.clone(),
            db_arc.clone(),
        ));
    }
}

fn ping_clients_with_new_tick(lobby: &mut models::Lobby, db_arc: models::DbArc) {
    lobby.tick = Uuid::new_v4();
    lobby.round += 1;

    if lobby.round >= MAX_ROUNDS {
        info!(
            "Maximum of rounds ({}) was reached, stopping lobby.",
            MAX_ROUNDS
        );
        lobby.status = models::LobbyStatus::FINISHED;
        return;
    }

    lobby.client_messages = HashMap::new();

    push_game_state_to_everyone(lobby, db_arc.clone());

    if get_amount_of_players_alive(lobby) <= 1 {
        info!("1 or less players alive, stopping lobby");
        lobby.status = models::LobbyStatus::FINISHED;
        return;
    }
}

fn get_amount_of_players_alive(lobby: &mut models::Lobby) -> usize {
    return lobby
        .game_state
        .players
        .values()
        .filter(|player| player.health > 0)
        .count();
}

fn push_game_state_to_everyone(lobby: &mut models::Lobby, db_arc: models::DbArc) {
    let socket_addresses: Vec<SocketAddr> = lobby.clients.keys().cloned().collect();

    push_game_state_to_addresses(lobby, socket_addresses, db_arc.clone());
}

fn push_game_state_to_spectators(lobby: &mut models::Lobby, db_arc: models::DbArc) {
    let socket_addresses: Vec<SocketAddr> = lobby
        .clients
        .iter()
        .filter_map(|(addr, client)| {
            if client.client_type == models::ClientType::SPECTATOR {
                return Some(addr);
            }
            return None;
        })
        .cloned()
        .collect();

    push_game_state_to_addresses(lobby, socket_addresses, db_arc.clone());
}

fn push_game_state_to_addresses(
    lobby: &mut models::Lobby,
    socket_addresses: Vec<SocketAddr>,
    db_arc: models::DbArc,
) {
    let game_state_out = get_game_state_out(lobby);

    for addr in socket_addresses {
        push_game_state_to_address(addr, &game_state_out, db_arc.clone());
    }
}

fn push_game_state_to_address(
    addr: SocketAddr,
    game_state_out: &api_models::GameStateOut,
    db_arc: models::DbArc,
) {
    info!("Pushing game state for tick '{}'", game_state_out.tick);
    let game_state_string = serde_json::to_string(game_state_out).unwrap();
    tokio::spawn(client_handling::send_message_to_addr(
        addr.clone(),
        tokio_tungstenite::tungstenite::Message::Text(game_state_string),
        db_arc.clone(),
    ));
}

fn get_game_state_out(lobby: &mut models::Lobby) -> api_models::GameStateOut {
    let spectator_count = lobby
        .clients
        .values()
        .filter(|client| client.client_type == models::ClientType::SPECTATOR)
        .count()
        .try_into()
        .unwrap();

    let game_state = lobby.game_state.clone();
    return api_models::GameStateOut {
        tick: lobby.tick,
        tick_length_milli_seconds: lobby.tick_length_milli_seconds,
        spectators: spectator_count,
        entities: game_state.entities,
        players: transform_map_of_players_to_list_of_player(game_state.players),
    };
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

    let mut server = server_arc.lock().await;
    let lobby = server.lobbies.get_mut(&lobby_id).unwrap();

    ping_clients_in_lobby(
        expected_tick,
        lobby_id,
        server_arc.clone(),
        db_arc.clone(),
        lobby,
    )
    .await;
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
) -> Result<Option<api_models::ClientHello>, String> {
    lobby.clients.insert(addr, new_client.clone());

    if new_client.client_type == models::ClientType::PLAYER {
        let player_id = Uuid::new_v4();
        let player_count = lobby.game_state.players.values().count();
        let color = PLAYER_COUNT_TO_COLOR.get(&(player_count + 1));

        if color.is_none() {
            return Err(format!(
                "Could not get color for new player. Lobby already has {} players.",
                player_count
            ));
        };

        let new_player = models::Player {
            entity_type: models::EntityType::PLAYER,
            id: player_id,
            name: new_client.username.clone(),
            x: 0,
            y: 0,
            rotation: 100,
            color: color.unwrap().to_string(),
            health: 100,
            last_action_success: true,
            error_message: "".to_string(),
        };

        lobby.game_state.players.insert(addr, new_player);

        let client_hello = match update_initial_player_positions(lobby) {
            Ok(()) => {
                push_game_state_to_spectators(lobby, db_arc.clone());
                api_models::ClientHello {
                    success: true,
                    player_id: player_id,
                    message: "Connection successful.".to_string(),
                }
            }

            Err(error_message) => {
                return Err(error_message);
            }
        };

        return Ok(Some(client_hello));
    }

    push_game_state_to_spectators(lobby, db_arc.clone());
    return Ok(None);
}

pub fn handle_client_disconnect(
    lobby: &mut models::Lobby,
    addr: SocketAddr,
    db_arc: models::DbArc,
) {
    let client_type = lobby.clients.get(&addr).unwrap().client_type.clone();

    lobby.clients.remove(&addr);

    info!("Client of type {:?} disconnected", client_type);

    if client_type == models::ClientType::PLAYER {
        lobby.game_state.players.remove(&addr);

        if lobby.status == models::LobbyStatus::PENDING {
            let _ = update_initial_player_positions(lobby);
        }

        ping_clients_with_new_tick(lobby, db_arc.clone());
    } else {
        push_game_state_to_spectators(lobby, db_arc.clone());
    }
}

fn handle_client_message(
    client_message: api_models::ClientMessage,
    addr: &SocketAddr,
    game_state: &mut models::GameState,
) {
    let player = game_state.players.get_mut(addr).unwrap();

    player.error_message = "".to_string();
    player.last_action_success = true;

    if player.health <= 0 {
        player.last_action_success = false;
        player.error_message =
            "Message was not processed, because player has no more health left".to_string();
        return;
    }

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
    lobby: &mut models::Lobby,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) {
    info!("Checking if all clients of lobby have responded");

    let expected_client_addresses: HashSet<SocketAddr> = lobby
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

    let clients_that_answered: HashSet<SocketAddr> =
        lobby.client_messages.keys().cloned().collect();

    if clients_that_answered.is_superset(&expected_client_addresses) {
        let mut db = db_arc.lock().await;

        let expected_tick = lobby.tick;

        info!(
            "Triggering premature lobby update, because all clients responded for tick '{}'",
            expected_tick
        );

        match db.open_tick_handles.get(&expected_tick) {
            Some(tick_handle) => {
                tick_handle.abort();
                db.open_tick_handles.remove(&expected_tick);
                info!("Canceled scheduled update for tick '{}'", expected_tick);
            }
            None => {
                info!("No scheduled update found for tick '{}'", expected_tick)
            }
        };

        ping_clients_in_lobby(
            expected_tick,
            lobby.id,
            server_arc.clone(),
            db_arc.clone(),
            lobby,
        )
        .await;
    }
}

fn calculate_projectile_updates(game_state: &mut models::GameState) {
    game_state.entities = game_state
        .entities
        .iter()
        .cloned()
        .filter(|projectile| {
            if projectile.x < 0.0 {
                return false;
            }
            if projectile.y < 0.0 {
                return false;
            }
            if projectile.x > MAX_FIELD_SIZE_X.into() {
                return false;
            }
            if projectile.y > MAX_FIELD_SIZE_Y.into() {
                return false;
            }
            return true;
        })
        .collect();

    game_state.entities.iter_mut().for_each(|projectile| {
        let list_of_hit_coordinates = get_fields_passed_by_projectile(projectile);

        game_state.players.iter_mut().for_each(|(addr, player)| {
            if list_of_hit_coordinates.contains(&(player.x, player.y))
                && projectile.source != addr.clone()
            {
                player.health = std::cmp::max(0, player.health - 20);
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
