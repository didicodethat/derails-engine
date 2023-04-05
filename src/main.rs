use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::net::{TcpListener, TcpStream};
use futures_util::{future, StreamExt, TryStreamExt};

use vector2::Vector2;
use game_map::{GameMap, WalkResult};
use serde::{Serialize, Deserialize};
use messages::{ServerMessages, ClientMessages, SimpleJSON, write_schemas_to_files};
mod two_way_range;
mod vector2;
mod game_map;
mod messages;

#[derive(Serialize, Deserialize)]
enum PlayerActions {
    MoveTo(Vector2),
    Idle
}

#[derive(Serialize, Deserialize)]
struct Player {
    position: Vector2,
    current_action: PlayerActions,
}

impl Player {
    fn new(position : Vector2) -> Self {
        return Self {
            position,
            current_action: PlayerActions::Idle
        }
    }
}

fn test_maps() {
    let file_path = "maps/starting.map";
    println!("In file {}", &file_path);
    let player = Player::new(Vector2(15, 5));
    let mut game_map = GameMap::from_file_path(&file_path, 24);
    println!("Player is at: {}", &player.position);
    println!("The tile where they're standing is: {}", game_map.on_position(&player.position));
    let target_position = Vector2(4,1);
    println!("Player can go from {} to {}?", &player.position, &target_position);
    game_map.plot(&player.position, &target_position, ' ');

    game_map.set_char(&player.position, '@');
    game_map.set_char(&target_position, 'O');

    match game_map.cast(&player.position, &target_position) {
        WalkResult::Finished(_) => println!("Could finish it's path"),
        WalkResult::Unfinished{hit, safe} => {
            println!("stopped at Position: {}", hit);
            println!("safe position is at: {}", safe);
            game_map.set_char(&safe, '=');
            game_map.set_char(&hit, 'X');
        }
    }
    println!("{}", &game_map.string_map);
}

#[tokio::main]
async fn main() {
    write_schemas_to_files();
    let file_path = "maps/starting.map";
    let mut game_map = GameMap::from_file_path(&file_path, 24);
    let addr = "127.0.0.1:8080".to_string();

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(accept_connection(stream));
    }
}
async fn accept_connection(stream: TcpStream) {
    let addr = stream.peer_addr().expect("connected streams should have a peer address");

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    let (write, mut read) = ws_stream.split();

    while let Some(message) = read.next().await {
        match message {
            Ok(message) => {
                if let Ok(message) = message.into_text() {
                    if let Ok(client_message) = ClientMessages::from_json(&message) {
                        match client_message {
                            ClientMessages::MoveTo(position) => println!("player wants to move to: {}", position),
                        }
                    } else {
                        println!("got message from client: {}", &message);
                    }   
                }
            },
            Err(_) => {
                println!("Couldn't read this message");
            }
        }
    }

    // We should not forward messages other than text or binary.
     
}