use uuid::Uuid;
use warp::http::StatusCode;

use std::collections::HashMap;
use std::convert::Infallible;

use warp::Filter;

use crate::api_models;
use crate::models;

fn with_server(
    server_arc: models::ServerArc,
) -> impl Filter<Extract = (models::ServerArc,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || server_arc.clone())
}

pub fn management_api(
    server_arc: models::ServerArc,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    list_lobbies(server_arc.clone())
        .or(create_lobby(server_arc.clone()))
        .or(update_lobby(server_arc.clone()))
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
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::path!("lobbies" / Uuid)
        .and(warp::patch())
        .and(with_server(server_arc.clone()))
        .and(warp::body::json())
        .and_then(get_update_lobby_reply)
}

async fn get_update_lobby_reply(
    lobby_id: Uuid,
    server_arc: models::ServerArc,
    update_lobby_body: api_models::UpdateLobbyBody,
) -> Result<impl warp::Reply, Infallible> {
    let mut server = server_arc.lock().await;

    server
        .lobbies
        .get_mut(&lobby_id)
        .expect(&format!("Lobby with id {} does not exist", lobby_id))
        .status = update_lobby_body.status;

    Ok(warp::reply::with_status("", StatusCode::OK))
}

async fn get_create_lobby_reply(
    server_arc: models::ServerArc,
) -> Result<impl warp::Reply, Infallible> {
    let mut server = server_arc.lock().await;

    let lobby_id = Uuid::new_v4();

    let new_lobby = models::Lobby {
        id: lobby_id,
        clients: HashMap::new(),
        status: models::LobbyStatus::PENDING,
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
