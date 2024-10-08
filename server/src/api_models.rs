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
    pub spectators: i32,
}

#[derive(Serialize)]
pub struct LobbyCreateResponse {
    pub id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLobbyBody {
    pub status: models::LobbyStatus,
}

#[derive(Serialize)]
pub struct GameStateOut {
    pub tick: Uuid,
    pub tick_length_milli_seconds: u64,
    pub players: Vec<models::Player>,
    pub entities: Vec<models::Projectile>,
    pub spectators: i32,
}

#[derive(Serialize)]
pub struct ClientHello {
    pub success: bool,
    pub message: String,
    pub player_id: Uuid,
}

#[derive(Deserialize, Clone)]
pub enum ClientAction {
    SHOOT,
    TURN,
    UP,
    DOWN,
    LEFT,
    RIGHT,
}

#[derive(Deserialize, Clone)]
pub struct ClientMessage {
    pub tick: Uuid,
    pub action: ClientAction,
    pub degrees: Option<i32>,
}
