use uuid::Uuid;
use warp::http::StatusCode;

use std::collections::HashMap;
use std::convert::Infallible;

use warp::Filter;

use crate::api_models;
use crate::game;
use crate::models;

fn with_server(
    server_arc: models::ServerArc,
) -> impl Filter<Extract = (models::ServerArc,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || server_arc.clone())
}

fn with_db(
    db_arc: models::DbArc,
) -> impl Filter<Extract = (models::DbArc,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || db_arc.clone())
}

pub fn management_api(
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let cors = warp::cors()
        .allow_any_origin()
        .allow_methods(vec!["GET", "POST", "PATCH"])
        .allow_headers(vec!["Content-Type", "Authorization"]);
    list_lobbies(server_arc.clone())
        .or(create_lobby(server_arc.clone()))
        .or(update_lobby(server_arc.clone(), db_arc.clone()))
        .with(cors)
}

fn list_lobbies(
    server_arc: models::ServerArc,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("lobbies")
        .and(warp::get())
        .and(with_server(server_arc.clone()))
        .and_then(get_lobbies_list_reply)
}

fn create_lobby(
    server_arc: models::ServerArc,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("lobbies")
        .and(warp::post())
        .and(with_server(server_arc.clone()))
        .and_then(get_create_lobby_reply)
}

fn update_lobby(
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("lobbies" / Uuid)
        .and(warp::patch())
        .and(with_server(server_arc.clone()))
        .and(with_db(db_arc.clone()))
        .and(warp::body::json())
        .and_then(get_update_lobby_reply)
}

async fn get_update_lobby_reply(
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    db_arc: models::DbArc,
    update_lobby_body: api_models::UpdateLobbyBody,
) -> Result<impl warp::Reply, Infallible> {
    let mut server = server_arc.lock().await;

    match server.lobbies.get_mut(&lobby_id) {
        Some(lobby) => {
            if matches!(lobby.status, models::LobbyStatus::RUNNING) {
                return Ok(warp::reply::with_status(
                    "Lobby status cannot be updated while lobby is running".to_string(),
                    StatusCode::UNPROCESSABLE_ENTITY,
                ));
            }

            if matches!(update_lobby_body.status, models::LobbyStatus::RUNNING) {
                tokio::spawn(game::start_game_for_lobby(
                    lobby_id,
                    server_arc.clone(),
                    db_arc.clone(),
                ));
            }

            lobby.status = update_lobby_body.status.clone();
            return Ok(warp::reply::with_status("".to_string(), StatusCode::OK));
        }
        None => {
            return Ok(warp::reply::with_status(
                format!("Lobby with id '{}' does not exist", lobby_id),
                StatusCode::NOT_FOUND,
            ))
        }
    }
}

async fn get_create_lobby_reply(
    server_arc: models::ServerArc,
) -> Result<impl warp::Reply, Infallible> {
    let mut server = server_arc.lock().await;

    let lobby_id = Uuid::new_v4();

    let new_lobby = models::Lobby {
        client_messages: HashMap::new(),
        id: lobby_id,
        clients: HashMap::new(),
        status: models::LobbyStatus::PENDING,
        game_state: None,
    };

    server.lobbies.insert(lobby_id, new_lobby);

    let new_lobby_reply = api_models::LobbyCreateResponse { id: lobby_id };

    Ok(warp::reply::json(&new_lobby_reply))
}

async fn get_lobbies_list_reply(
    server_arc: models::ServerArc,
) -> Result<impl warp::Reply, Infallible> {
    let server = server_arc.lock().await;

    let server_out = api_models::ServerOut {
        lobbies: server
            .lobbies
            .values()
            .cloned()
            .map(|lobby| api_models::LobbyOut {
                status: lobby.status,
                id: lobby.id,
                clients: lobby.clients.values().cloned().collect(),
            })
            .collect(),
    };

    Ok(warp::reply::json(&server_out))
}
