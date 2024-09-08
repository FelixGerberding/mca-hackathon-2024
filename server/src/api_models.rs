use serde::Serialize;
use uuid::Uuid;

use crate::models::Client;

#[derive(Serialize)]
pub struct ServerOut {
    pub lobbies: Vec<LobbyOut>,
}

#[derive(Serialize)]
pub struct LobbyOut {
    pub id: Uuid,
    pub clients: Vec<Client>,
}

#[derive(Serialize)]
pub struct LobbyCreateResponse {
    pub id: Uuid,
}
