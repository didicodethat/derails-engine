use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, Duration};
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

type PlayerSink = futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<TcpStream>, tokio_tungstenite::tungstenite::Message>;

type WebsocketError = tokio_tungstenite::tungstenite::Error;

#[derive(Copy, Clone, Debug)]
enum PlayerActions {
    MoveTo(Vector2),
    Idle,
    Disconnect,
}

#[derive(Copy, Clone, Debug)]
struct PlayerAction {
    id: u32,
    action: PlayerActions
}

impl PlayerAction {
    pub fn disconnect(id: u32) -> Self {
        Self {id, action: PlayerActions::Disconnect}
    }
}


struct PlayerConnected {
    id: u32,
    sink: PlayerSink
}

struct Player {
    position: Vector2,
    current_action: PlayerActions,
}

struct PlayerConnection {
    sink: PlayerSink,
    player: Player
}

impl PlayerConnection {
    fn new(sink: PlayerSink) -> Self {
        Self {
            sink,
            player: Player{
                position: Vector2(0, 0),
                current_action : PlayerActions::Idle
            }
        }
    }
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
    let (player_action_sender, mut player_action_receiver) = mpsc::channel::<PlayerAction>(32);
    let (player_connect_sender, mut player_connect_receiver) = mpsc::channel::<PlayerConnected>(32);

    tokio::spawn(async move {game_logic(&mut player_connect_receiver, &mut player_action_receiver).await});
    let mut player_connection_identifier = 0u32;
    while let Ok((stream, _)) = listener.accept().await {
        player_connection_identifier += 1;
        tokio::spawn(accept_connection(
            player_connection_identifier, 
            stream, 
            player_connect_sender.to_owned(),
            player_action_sender.to_owned()
        ));
    }
}
async fn accept_connection(id: u32, stream: TcpStream, send_connect: Sender<PlayerConnected>, send_action: Sender<PlayerAction>) {
    let addr = stream.peer_addr().expect("connected streams should have a peer address");

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    let (write, mut read) = ws_stream.split();
    send_connect.send(PlayerConnected { id, sink: write });
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
            Err(e) => {
                match e {
                    WebsocketError::AlreadyClosed => {
                        // close connection
                        send_action.send(PlayerAction::disconnect(id));
                    },
                    _ => print!("got a unhandable error"),
                }
            }
        }
    }

    // We should not forward messages other than text or binary.
     
}

async fn game_logic(player_connections : &mut Receiver<PlayerConnected>, player_actions : &mut Receiver<PlayerAction>) {
    let mut connected_players : HashMap<u32, PlayerConnection> = HashMap::new();
    loop {
        //let broadcast_responses = Vec::new();

        // add new players to the connection hashmap
        while let Some(connection) = player_connections.recv().await {
            connected_players.insert(connection.id, PlayerConnection::new(connection.sink));
        }

        // set their actions
        while let Some(action) = player_actions.recv().await {
            let entry = connected_players.get_mut(&action.id);
            if let Some(player_connection) = entry {
                player_connection.player.current_action = action.action;
            }
        }

        // execute their actions
        for mut connection in connected_players.values() {
            
        }

        // send global actions
        for mut connection in connected_players.values() {

        }

        sleep(Duration::from_millis(1000)).await; 
    }
}