use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use std::{env, io::Error};

use futures_util::stream::{self, SplitStream};
use futures_util::{future, pin_mut, SinkExt, StreamExt, TryStreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::{self};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;

use log::info;
use querystring;
use rand::Rng;
use regex::Regex;

use uuid::Uuid;
use warp::Filter;

mod api_models;
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

    let interval = time::interval(Duration::from_millis(500));

    let forever = stream::unfold(interval, |mut interval| async {
        interval.tick().await;
        send_messages_to_clients(lobby_id, db_arc.clone(), server_arc.clone()).await;
        Some(((), interval))
    });

    // let now = Instant::now();
    let ping_clients = forever.for_each(|_| async {});
    pin_mut!(ping_clients);

    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let getLobbies = warp::path!("lobbies")
        .and(warp::get())
        .and(with_server(server_arc.clone()))
        .and_then(returnLobbies);

    let rest_api = warp::serve(getLobbies).run(([127, 0, 0, 1], 8081));
    pin_mut!(rest_api);

    future::select(ping_clients, rest_api).await;

    loop {}
}

fn with_server(
    server_arc: models::ServerArc,
) -> impl Filter<Extract = (models::ServerArc,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || server_arc.clone())
}

async fn returnLobbies(server_arc: models::ServerArc) -> Result<impl warp::Reply, Infallible> {
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
            clientType: client_type,
            username: username.to_string(),
            addr: addr,
        };

        let new_connection = models::Connection {
            write_stream: write,
        };

        let mut db = db_arc.lock().await;
        let mut server = server_arc.lock().await;

        server
            .lobbies
            .get_mut(&lobby_uuid)
            .unwrap()
            .clients
            .insert(addr, new_client);

        db.connections.insert(addr, new_connection);

        info!(
            "Added client to lobby. List of clients: {:?}",
            server.lobbies.get(&lobby_uuid).unwrap().clients
        );

        tokio::spawn(listen_for_messages(
            read,
            addr,
            lobby_uuid,
            db_arc.clone(),
            server_arc.clone(),
        ));
    }
}

async fn listen_for_messages(
    readStream: SplitStream<WebSocketStream<TcpStream>>,
    addr: SocketAddr,
    lobby_id: Uuid,
    db_arc: models::DbArc,
    server_arc: models::ServerArc,
) {
    let broadcast_incoming = readStream.try_for_each(|msg| {
        println!(
            "Received a message from {}: {}",
            addr,
            msg.to_text().unwrap()
        );

        future::ok(())
    });

    pin_mut!(broadcast_incoming);
    broadcast_incoming.await;

    println!("{} disconnected", addr);

    let mut server = server_arc.lock().await;
    server
        .lobbies
        .get_mut(&lobby_id)
        .unwrap()
        .clients
        .remove(&addr);

    let mut db = db_arc.lock().await;
    db.connections.remove(&addr);
}

async fn send_messages_to_clients(
    lobby_id: Uuid,
    db_arc: models::DbArc,
    server_arc: models::ServerArc,
) {
    let random_game_state = get_random_get_state();

    let mut server = server_arc.lock().await;

    let addresses: Vec<SocketAddr> = server
        .lobbies
        .get_mut(&lobby_id)
        .unwrap()
        .clients
        .values()
        .map(|client| client.addr)
        .collect();

    info!("Sending messages to: {:?}", addresses);

    let send_futures: Vec<_> = addresses
        .iter()
        .map(|addr| {
            send_message(
                &addr,
                Message::text(serde_json::to_string(&random_game_state).unwrap()),
                db_arc.clone(),
            )
        })
        .collect();

    for future in send_futures {
        future.await;
    }
}

async fn send_message(addr: &SocketAddr, message: Message, db_arc: models::DbArc) {
    let mut db = db_arc.lock().await;

    db.connections
        .get_mut(addr)
        .expect(&format!(
            "No connection found for client with address '{}'",
            addr
        ))
        .write_stream
        .send(message)
        .await
        .expect("Sending failed");
}

fn get_random_get_state() -> models::GameState {
    let player = models::Player {
        entityType: models::EntityType::PLAYER,
        color: "#FF0000".to_string(),
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        errorMessage: "".to_string(),
        health: 100,
        lastActionSuccess: true,
        rotation: rand::thread_rng().gen_range(0..360),
        x: rand::thread_rng().gen_range(0..40),
        y: rand::thread_rng().gen_range(0..40),
    };

    return models::GameState {
        entities: vec![player],
    };
}
