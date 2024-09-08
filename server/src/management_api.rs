use uuid::Uuid;

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
    list_lobbies(server_arc.clone()).or(create_lobby(server_arc.clone()))
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

async fn get_create_lobby_reply(
    server_arc: models::ServerArc,
) -> Result<impl warp::Reply, Infallible> {
    let mut server = server_arc.lock().await;

    let lobby_id = Uuid::new_v4();

    let new_lobby = models::Lobby {
        id: lobby_id,
        clients: HashMap::new(),
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
                id: lobby.id,
                clients: lobby.clients.values().cloned().collect(),
            })
            .collect(),
    };

    Ok(warp::reply::json(&server_out))
}
