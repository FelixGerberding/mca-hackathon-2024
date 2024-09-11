use futures_util::stream::SplitSink;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;

use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Serialize, Clone)]
pub enum ClientType {
    PLAYER,
    SPECTATOR,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum LobbyStatus {
    PENDING,
    RUNNING,
    FINISHED,
}

impl FromStr for ClientType {
    type Err = ();

    fn from_str(input: &str) -> Result<ClientType, Self::Err> {
        match input {
            "PLAYER" => Ok(ClientType::PLAYER),
            "SPECTATOR" => Ok(ClientType::SPECTATOR),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Clone)]
pub enum EntityType {
    PLAYER,
}

#[derive(Serialize, Clone)]
pub struct Projectile {}

#[derive(Serialize, Clone)]
pub struct Player {
    pub entity_type: EntityType,
    pub id: Uuid,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub rotation: i32,
    pub color: String,
    pub health: i16,
    pub last_action_success: bool,
    pub error_message: String,
}

#[derive(Serialize, Clone)]
pub struct GameState {
    pub players: HashMap<SocketAddr, Player>,
    pub entities: Vec<Projectile>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Client {
    pub client_type: ClientType,
    pub username: String,
}

#[derive(Clone)]
pub struct Lobby {
    pub id: Uuid,
    pub clients: HashMap<SocketAddr, Client>,
    pub status: LobbyStatus,
    pub game_state: Option<GameState>,
}

pub struct Server {
    pub lobbies: HashMap<Uuid, Lobby>,
}

pub type ServerArc = Arc<Mutex<Server>>;

pub struct Connection {
    pub write_stream: SplitSink<WebSocketStream<TcpStream>, Message>,
}

pub struct Db {
    pub connections: HashMap<SocketAddr, Connection>,
}

pub type DbArc = Arc<Mutex<Db>>;
