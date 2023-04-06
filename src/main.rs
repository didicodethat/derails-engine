use futures_util::{StreamExt};
use player::{PlayerAction, PlayerConnected};
use std::collections::HashMap;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use futures_util::SinkExt;
use std::sync::{Arc, Mutex};

use game_map::{GameMap, WalkResult};
use messages::{
    write_schemas_to_files, ClientMessages, PlayerBroadcastAction, ServerMessages, SimpleJSON,
};
use tokio_tungstenite::tungstenite::Message;
use vector2::Vector2;

use crate::player::{Player, PlayerConnection};
mod game_map;
mod messages;
mod two_way_range;
mod vector2;
mod player;

type WebsocketError = tokio_tungstenite::tungstenite::Error;


fn test_maps() {
    let file_path = "maps/starting.map";
    println!("In file {}", &file_path);
    let player = Player::new(Vector2(15, 5));
    let mut game_map = GameMap::from_file_path(&file_path, 24);
    println!("Player is at: {}", &player.position);
    println!(
        "The tile where they're standing is: {}",
        game_map.on_position(&player.position)
    );
    let target_position = Vector2(4, 1);
    println!(
        "Player can go from {} to {}?",
        &player.position, &target_position
    );
    game_map.plot(&player.position, &target_position, ' ');

    game_map.set_char(&player.position, '@');
    game_map.set_char(&target_position, 'O');

    match game_map.cast(&player.position, &target_position) {
        WalkResult::Finished(_) => println!("Could finish it's path"),
        WalkResult::Unfinished { hit, safe } => {
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

    tokio::spawn(async move {
        game_logic(&mut player_connect_receiver, &mut player_action_receiver).await
    });

    let mut player_connection_identifier = 0u32;
    while let Ok((stream, addr)) = listener.accept().await {
        player_connection_identifier += 1;
        tokio::spawn(accept_player_connection(
            player_connection_identifier,
            stream,
            player_connect_sender.to_owned(),
            player_action_sender.to_owned(),
        ));
    }
}
async fn accept_player_connection(
    id: u32,
    stream: TcpStream,
    send_connect: Sender<PlayerConnected>,
    send_action: Sender<PlayerAction>,
) {
    let addr = stream
        .peer_addr()
        .expect("connected streams should have a peer address");

    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    let (write, mut read) = ws_stream.split();
    send_connect
        .send(PlayerConnected::new(id, write))
        .await
        .unwrap();

    while let Some(message) = read.next().await {
        match message {
            Ok(message) => {
                if let Ok(message) = message.into_text() {
                    if let Ok(client_message) = ClientMessages::from_json(&message) {
                        match client_message {
                            ClientMessages::MoveTo(position) => {
                                println!("sending a moveto message");
                                send_action
                            .send(PlayerAction::move_to(id, position))
                            .await
                            .unwrap();
                            }
                        }
                    } else {
                        println!("got message from client: {}", &message);
                    }
                }
            }
            Err(e) => {
                match e {
                    WebsocketError::AlreadyClosed => {
                        // close connection
                        send_action
                            .send(PlayerAction::disconnect(id))
                            .await
                            .unwrap();
                    }
                    _ => print!("got a unhandable error"),
                }
            }
        }
    }

    // We should not forward messages other than text or binary.
}

async fn game_logic<'a, 'b>(
    player_connections: &mut Receiver<PlayerConnected>,
    player_actions: &mut Receiver<PlayerAction>,
) {
    let mut connected_players: HashMap<u32, PlayerConnection> = HashMap::new();
 
    
    loop {
        let mut broadcast_responses: Vec<ServerMessages> = Vec::new();
        let mut dead_connections : Vec<u32> = Vec::new();
        
        while let Ok(connection) = player_connections.try_recv() {
            let player_connection = PlayerConnection::new(connection.sink);
            broadcast_responses.push(ServerMessages::PlayerConnected(
                connection.id,
                player_connection.player.position.clone(),
            ));
            connected_players.insert(connection.id, player_connection);
        }

        while let Ok(action) = player_actions.try_recv() {
            let entry = connected_players.get_mut(&action.id);
            if let Some(player_connection) = entry {
                player_connection
                    .player
                    .update_action(action.action.clone());
            }
        }

        // execute their actions
        for (id, connection) in connected_players.iter_mut() {
            if let Some(action) = connection.player.next() {
                if let PlayerBroadcastAction::None = action {
                    continue;
                }
                broadcast_responses.push(ServerMessages::BroadCastAction(*id, action));
            }
        }

        // send global actions
        for (id, mut connection) in connected_players.iter_mut() {
            for respose in &broadcast_responses {
                let result = connection
                    .sink
                    .send(Message::Text(ServerMessages::to_json(&respose).unwrap()))
                    .await;


                if let Err(result) = result {
                    match result {
                        tokio_tungstenite::tungstenite::Error::ConnectionClosed
                        | tokio_tungstenite::tungstenite::Error::AlreadyClosed => {
                            dead_connections.push(*id);
                        }
                        _ => {
                            println!(
                                "couldn't send a answer to a user, but connection isn't dead yet"
                            );
                        }
                    }
                }
            }
        }

        for id in &dead_connections {
            connected_players.remove(id);
        }

        let mut disconnect_messages = Vec::new();
        for id in &dead_connections {
            let message = ServerMessages::PlayerDisconnected(*id);
            if let Ok(string) = ServerMessages::to_json(&message) {
                disconnect_messages.push(string);
            }
        }

        for message in &disconnect_messages {
            for (_, connection) in connected_players.iter_mut() {
                // Don't care if this message fails, we handle them next time.
                let _ = connection.sink.send(Message::text(message)).await;
            }
        }

        sleep(Duration::from_millis(1000)).await;
    }
}
