use std::fs::File;

use futures_util::{SinkExt, StreamExt};
use game_map::{GameMap, GameMapServer};
use messages::{write_schemas_to_files, ServerMessages, SimpleJSON};
use player::{PlayerConnection, PlayerConnectionId};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::{self, Sender};
use tokio::time::{sleep, Duration};
use tokio_tungstenite::tungstenite::Message;
mod game_map;
mod messages;
mod player;
mod two_way_range;
mod vector2;

pub type WebsocketError = tokio_tungstenite::tungstenite::Error;

#[tokio::main]
async fn main() {
    write_schemas_to_files();
    let addr = "127.0.0.1:8080".to_string();
    let mut maps = vec![("maps/starting.map", 24)]
        .into_iter()
        .map(|(filename, width)| GameMap::from_file_path(filename, width))
        .enumerate()
        .map(|(id, map)| GameMapServer::new(id, map))
        .collect::<Vec<GameMapServer>>();

    let mut starting_map = maps.pop().unwrap();
    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");

    let funnel = starting_map.connection_funnel.to_owned();

    tokio::spawn(async move {
        let mut player_connection_identifier: PlayerConnectionId = 0;
        while let Ok((stream, addr)) = listener.accept().await {
            player_connection_identifier += 1;
            tokio::spawn(accept_player_connection(
                player_connection_identifier,
                stream,
                funnel.to_owned(),
            ));
        }
    });

    loop {
        starting_map.step().await;
        sleep(Duration::from_millis(300)).await
    }

    // on connect we choose which map the player should connect to
}
async fn accept_player_connection(
    id: PlayerConnectionId,
    stream: TcpStream,
    player_connected: Sender<PlayerConnection>,
) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    if let Err(mut error) = player_connected
        .send(PlayerConnection::new(id, ws_stream))
        .await
    {
        error
            .0
            .sink
            .send(Message::text(
                serde_json::to_string(&ServerMessages::connetion_error()).unwrap(),
            ))
            .await
            .unwrap();
    }
}
