use std::borrow::Cow;
use std::collections::HashMap;

use std::str::FromStr;
use std::sync::Arc;
use std::{env, io::Error};

use futures_util::{pin_mut, SinkExt, StreamExt};
use models::Connection;
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
        .unwrap_or_else(|| "127.0.0.1".to_string());

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&format!("{addr}:8080")).await;
    let listener = try_socket.expect("Failed to bind");

    info!("Listening on: {}:8080", addr);

    let mut server = models::Server {
        lobbies: HashMap::new(),
    };

    // let lobby_id = Uuid::new_v4();
    let lobby_id = Uuid::parse_str("9ec2a984-b5bf-4a13-89fd-53c0d9cafef6").unwrap();

    let lobby = models::Lobby {
        round: 0,
        tick: Uuid::new_v4(),
        tick_length_milli_seconds: models::TICK_LENGTH_MILLI_SECONDS,
        client_messages: HashMap::new(),
        clients: HashMap::new(),
        id: lobby_id,
        status: models::LobbyStatus::PENDING,
        game_state: models::GameState {
            players: HashMap::new(),
            entities: Vec::new(),
        },
    };

    info!("Lobby created with id: {}", lobby.id);

    server.lobbies.insert(lobby.id, lobby);

    let server_arc = Arc::new(Mutex::new(server));

    let db = models::Db {
        open_tick_handles: HashMap::new(),
        connections: HashMap::new(),
    };

    let db_arc = Arc::new(Mutex::new(db));

    tokio::spawn(listen_for_connections(
        listener,
        db_arc.clone(),
        server_arc.clone(),
    ));

    let address_ip_parts: [u8; 4] = addr
        .split(".")
        .map(|str| str.parse::<u8>().unwrap())
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap();

    let rest_api = warp::serve(management_api::management_api(
        server_arc.clone(),
        db_arc.clone(),
    ))
    .run((address_ip_parts, 8081));
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
        info!("New connection incoming");
        let addr = stream
            .peer_addr()
            .expect("connected streams should have a peer address");

        let mut buffer = [0; 2048];
        stream.peek(&mut buffer).await.expect("Failed to peek");
        let request_str = std::str::from_utf8(&buffer).unwrap();

        let lines: Vec<String> = request_str.lines().map(|line| line.to_string()).collect();
        let request_line = lines.first().unwrap().to_string();

        let ws_stream = tokio_tungstenite::accept_async(stream)
            .await
            .expect("Error during the websocket handshake occurred");

        let (write, read) = ws_stream.split();

        let mut new_connection = models::Connection {
            write_stream: write,
        };

        info!("New request: {}", request_line);
        let request_parts: Vec<&str> = request_line.split(" ").collect();

        let request_url = request_parts.get(1).expect("Could not get URL of request");

        let request_regex = Regex::new(r"^\/lobby\/(.*)\?(.*)").unwrap();

        let mut results = vec![];

        for (_, [lobby_id, query_string]) in request_regex
            .captures_iter(&request_url)
            .map(|c| c.extract())
        {
            results.push(lobby_id);
            results.push(query_string)
        }

        info!("regex matches: {:?}", results);

        let lobby_id_str_option = results.get(0);

        if lobby_id_str_option.is_none() {
            close_connection(
                &mut new_connection,
                "Could not find lobby id in path".to_string(),
            )
            .await;
            continue;
        }

        let lobby_id_str = lobby_id_str_option.unwrap();

        let lobby_uuid_result = Uuid::parse_str(lobby_id_str);

        if lobby_uuid_result.is_err() {
            close_connection(
                &mut new_connection,
                format!("'{}' is not a valid UUID", lobby_id_str),
            )
            .await;
            continue;
        }

        let lobby_uuid = lobby_uuid_result.unwrap();

        let query_string_option = results.get(1);

        if query_string_option.is_none() {
            close_connection(
                &mut new_connection,
                "Missing query string in URL".to_string(),
            )
            .await;
            continue;
        }

        let query_string = query_string_option.unwrap();

        let query_params = querystring::querify(query_string)
            .into_iter()
            .collect::<HashMap<&str, &str>>();

        let client_type_str_option = query_params.get("clientType");

        if client_type_str_option.is_none() {
            close_connection(
                &mut new_connection,
                "Missing 'clientType' parameter in supplied query parameters".to_string(),
            )
            .await;
            continue;
        }

        let client_type_str = client_type_str_option.unwrap();

        let client_type_result = models::ClientType::from_str(&client_type_str);
        if client_type_result.is_err() {
            close_connection(
                &mut new_connection,
                format!("{} is not a valid client type", client_type_str),
            )
            .await;
            continue;
        }

        let client_type = client_type_result.unwrap();

        let username = query_params.get("username").unwrap_or(&"");

        if client_type == models::ClientType::PLAYER && username.to_string() == "" {
            close_connection(
                &mut new_connection,
                "Player clients must supply a 'username' via the query parameter".to_string(),
            )
            .await;
            continue;
        }

        info!(
            "Extract the following info from request. Lobby id: '{}', client type: {}, username: {}",
            lobby_uuid, client_type_str, username
        );

        info!("New WebSocket connection: {}", addr);

        let new_client = models::Client {
            client_type: client_type,
            username: username.to_string(),
        };

        let mut server = server_arc.lock().await;

        let lobby_option = server.lobbies.get_mut(&lobby_uuid);

        if lobby_option.is_none() {
            close_connection(
                &mut new_connection,
                format!("Could not find lobby with id '{}'", lobby_uuid),
            )
            .await;
            continue;
        }

        let lobby = lobby_option.unwrap();

        if matches!(new_client.client_type, models::ClientType::SPECTATOR)
            || (matches!(new_client.client_type, models::ClientType::PLAYER)
                && matches!(lobby.status, models::LobbyStatus::PENDING))
        {
            let mut db = db_arc.lock().await;

            match game::handle_client_connect(lobby, addr, new_client.clone(), db_arc.clone()).await
            {
                Ok(client_hello) => {
                    db.connections.insert(addr, new_connection);

                    if new_client.client_type == models::ClientType::PLAYER {
                        let message = Message::Text(serde_json::to_string(&client_hello).unwrap());

                        tokio::spawn(client_handling::send_message_to_addr(
                            addr,
                            message,
                            db_arc.clone(),
                        ));
                    }

                    tokio::spawn(client_handling::listen_for_messages(
                        read,
                        addr,
                        lobby_uuid,
                        db_arc.clone(),
                        server_arc.clone(),
                    ));
                }

                Err(error_message) => {
                    close_connection(&mut new_connection, error_message).await;
                    continue;
                }
            };
        } else {
            close_connection(
                &mut new_connection,
                format!(
                    "Lobby with id '{}' is not open for new connections",
                    lobby_id_str
                ),
            )
            .await;
            continue;
        }
    }
}

async fn close_connection(connection: &mut Connection, error_reason: String) {
    info!("Declining connection, due to error: {}", error_reason);

    connection
        .write_stream
        .send(Message::Close(Some(CloseFrame {
            code: CloseCode::Normal,
            reason: Cow::Owned(error_reason),
        })))
        .await
        .expect("Closing connection failed");

    info!("Connection closed");
}
