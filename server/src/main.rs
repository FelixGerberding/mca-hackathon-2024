use std::borrow::Cow;
use std::collections::HashMap;

use std::str::FromStr;
use std::sync::Arc;
use std::{env, io::Error};

use futures_util::{pin_mut, SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;

use log::info;
use querystring;
use regex::Regex;

use uuid::Uuid;

mod api_models;
mod client_handling;
mod game;
mod management_api;
mod models;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = env_logger::try_init();
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");

    info!("Listening on: {}", addr);

    let mut server = models::Server {
        lobbies: HashMap::new(),
    };

    // let lobby_id = Uuid::new_v4();
    let lobby_id = Uuid::parse_str("9ec2a984-b5bf-4a13-89fd-53c0d9cafef6").unwrap();

    let lobby = models::Lobby {
        clients: HashMap::new(),
        id: lobby_id,
        status: models::LobbyStatus::PENDING,
        game_state: None,
    };

    info!("Lobby created with id: {}", lobby.id);

    server.lobbies.insert(lobby.id, lobby);

    let server_arc = Arc::new(Mutex::new(server));

    let db = models::Db {
        connections: HashMap::new(),
    };

    let db_arc = Arc::new(Mutex::new(db));

    tokio::spawn(listen_for_connections(
        listener,
        db_arc.clone(),
        server_arc.clone(),
    ));

    let rest_api = warp::serve(management_api::management_api(
        server_arc.clone(),
        db_arc.clone(),
    ))
    .run(([127, 0, 0, 1], 8081));
    pin_mut!(rest_api);

    rest_api.await;
    Ok(())
}

async fn listen_for_connections(
    listener: TcpListener,
    db_arc: models::DbArc,
    server_arc: models::ServerArc,
) {
    while let Ok((stream, _)) = listener.accept().await {
        let addr = stream
            .peer_addr()
            .expect("connected streams should have a peer address");

        let mut buffer = [0; 2048];
        stream.peek(&mut buffer).await.expect("Failed to peek");
        let request_str = std::str::from_utf8(&buffer).unwrap();

        let lines: Vec<String> = request_str.lines().map(|line| line.to_string()).collect();
        let request_line = lines.first().unwrap().to_string();

        info!("New request: {}", request_line);
        let request_parts: Vec<&str> = request_line.split(" ").collect();

        let request_url = request_parts.get(1).expect("Could not get URL of request");

        let requestRegex = Regex::new(r"^\/lobby\/(.*)\?(.*)").unwrap();

        let mut results = vec![];

        for (_, [lobby_id, query_string]) in requestRegex
            .captures_iter(&request_url)
            .map(|c| c.extract())
        {
            results.push(lobby_id);
            results.push(query_string)
        }

        info!("regex matches: {:?}", results);

        let lobby_id_str = results
            .get(0)
            .expect("Could not get lobby id from request matches");
        let lobby_uuid = Uuid::parse_str(lobby_id_str).unwrap();
        let queryParams =
            querystring::querify(results.get(1).expect("Request is missing query parameters"))
                .into_iter()
                .collect::<HashMap<&str, &str>>();

        let client_type_str = queryParams
            .get("clientType")
            .expect("Missing client type from supplied query parameters");

        let client_type = models::ClientType::from_str(&client_type_str).unwrap();
        let username = queryParams.get("username").unwrap_or(&"");

        info!(
            "Extract the following info from request. Lobby id: '{}', client type: {}, username: {}",
            lobby_uuid, client_type_str, username
        );

        info!("Peer address: {}", addr);

        let ws_stream = tokio_tungstenite::accept_async(stream)
            .await
            .expect("Error during the websocket handshake occurred");

        info!("New WebSocket connection: {}", addr);

        let (write, read) = ws_stream.split();

        let new_client = models::Client {
            client_type: client_type,
            username: username.to_string(),
            addr: addr,
        };

        let mut new_connection = models::Connection {
            write_stream: write,
        };

        let mut server = server_arc.lock().await;

        info!(
            "Lobby is currently in state: '{:?}'",
            server.lobbies.get(&lobby_uuid).unwrap().status
        );

        if matches!(
            server.lobbies.get(&lobby_uuid).unwrap().status,
            models::LobbyStatus::PENDING
        ) {
            server
                .lobbies
                .get_mut(&lobby_uuid)
                .unwrap()
                .clients
                .insert(addr, new_client);

            let mut db = db_arc.lock().await;

            db.connections.insert(addr, new_connection);

            info!(
                "Added client to lobby. List of clients: {:?}",
                server.lobbies.get(&lobby_uuid).unwrap().clients
            );

            tokio::spawn(client_handling::listen_for_messages(
                read,
                addr,
                lobby_uuid,
                db_arc.clone(),
                server_arc.clone(),
            ));
        } else {
            let error_reason = format!(
                "Lobby with id '{}' is not open for new connections",
                lobby_id_str
            );

            new_connection
                .write_stream
                .send(Message::Close(Some(CloseFrame {
                    code: CloseCode::Normal,
                    reason: Cow::Owned(error_reason),
                })))
                .await
                .expect("Closing connection failed");
        }
    }
}
