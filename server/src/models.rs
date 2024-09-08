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

#[derive(Serialize, Deserialize)]
pub enum EntityType {
    PLAYER,
}

#[derive(Serialize, Deserialize)]
pub struct Player {
    pub entityType: EntityType,
    pub id: Uuid,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub rotation: i32,
    pub color: String,
    pub health: i16,
    pub lastActionSuccess: bool,
    pub errorMessage: String,
}

#[derive(Serialize, Deserialize)]
pub struct GameState {
    pub entities: Vec<Player>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Client {
    pub clientType: ClientType,
    pub username: String,
    #[serde(skip)]
    pub addr: SocketAddr,
}

#[derive(Clone)]
pub struct Lobby {
    pub id: Uuid,
    pub clients: HashMap<SocketAddr, Client>,
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
