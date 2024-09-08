use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models;

#[derive(Serialize)]
pub struct ServerOut {
    pub lobbies: Vec<LobbyOut>,
}

#[derive(Serialize)]
pub struct LobbyOut {
    pub id: Uuid,
    pub clients: Vec<models::Client>,
    pub status: models::LobbyStatus,
}

#[derive(Serialize)]
pub struct LobbyCreateResponse {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLobbyBody {
    pub status: models::LobbyStatus,
}
