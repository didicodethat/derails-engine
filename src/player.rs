use std::{sync::Arc, thread::JoinHandle};

use crate::{
    game_map::{GameMap, WalkResult},
    messages::{ClientMessages, PlayerBroadcastAction, SimpleJSON},
    vector2::{Vector2, Vector2Range},
    WebsocketError,
};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::{mpsc::Sender, Mutex},
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

pub type PlayerSink = futures_util::stream::SplitSink<WebSocketStream<TcpStream>, Message>;

pub type PlayerStream = futures_util::stream::SplitStream<WebSocketStream<TcpStream>>;

pub type PlayerConnectionId = usize;

#[derive(Copy, Clone, Debug)]
pub enum PlayerActions {
    MoveTo(Vector2),
    Idle,
    Disconnect,
}

#[derive(Copy, Clone, Debug)]
pub struct PlayerAction {
    pub id: PlayerConnectionId,
    pub action: PlayerActions,
}

impl PlayerAction {
    pub fn disconnect(id: PlayerConnectionId) -> Self {
        Self {
            id,
            action: PlayerActions::Disconnect,
        }
    }

    pub fn move_to(id: PlayerConnectionId, position: Vector2) -> Self {
        Self {
            id,
            action: PlayerActions::MoveTo(position),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Player {
    pub position: Vector2,
    pub current_action: PlayerActions,
    pub current_path: Option<Vector2Range>,
}

impl Player {
    pub fn new(position: Vector2) -> Self {
        Self {
            position,
            current_action: PlayerActions::Idle,
            current_path: None,
        }
    }

    pub fn update_action(&mut self, action: PlayerActions, map: Option<&GameMap>) {
        self.current_action = action;
        if let PlayerActions::MoveTo(target_position) = action {
            if let Some(map) = map {
                let target = match map.cast(&self.position, &target_position) {
                    WalkResult::Finished(position) => position,
                    WalkResult::Unfinished { hit, safe } => safe,
                };
                self.current_path = Some(Vector2Range::new(&self.position, &target));
            } else {
                unreachable!();
            }
        }
    }
}

impl Iterator for Player {
    type Item = PlayerBroadcastAction;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current_action {
            PlayerActions::MoveTo(_) => match self.current_path {
                None => {
                    self.update_action(PlayerActions::Idle, None);
                    Some(PlayerBroadcastAction::None)
                }
                Some(mut path) => match path.next() {
                    Some(step) => {
                        self.position = step;
                        self.current_path = Some(path);
                        Some(PlayerBroadcastAction::Step(step))
                    }
                    None => {
                        self.update_action(PlayerActions::Idle, None);
                        Some(PlayerBroadcastAction::None)
                    }
                },
            },
            PlayerActions::Idle => Some(PlayerBroadcastAction::None),
            PlayerActions::Disconnect => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AlreadyReadingMessages;

#[derive(Debug)]
pub struct PlayerConnection {
    pub id: PlayerConnectionId,
    pub sink: PlayerSink,
    pub stream: Arc<Mutex<PlayerStream>>,
    pub player: Player,
    pub join_handle: Option<JoinHandle<()>>
}

impl PlayerConnection {
    pub fn new(id: PlayerConnectionId, connection: WebSocketStream<TcpStream>) -> Self {
        let (sink, stream) = connection.split();
        Self {
            id,
            sink,
            stream: Arc::new(Mutex::new(stream)),
            player: Player::new(Vector2(0, 0)),
            join_handle: None
        }
    }

    pub async fn send_message(
        &mut self,
        message: Message,
    ) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        self.sink.send(message).await
    }

    pub fn read_messages(&mut self, send_action: &Sender<PlayerAction>) -> Result<(), AlreadyReadingMessages> {
        match self.join_handle {
            Some(_) => Err(AlreadyReadingMessages),
            None => {
                let stream = self.stream.clone();
                let id = self.id;
                let action = send_action.to_owned();
                tokio::spawn(async move {
                    let mut incoming = stream.lock().await;
                    while let Some(message) = incoming.next().await {
                        match message {
                            Ok(message) => {
                                if let Ok(message) = message.into_text() {
                                    if let Ok(client_message) = ClientMessages::from_json(&message) {
                                        match client_message {
                                            ClientMessages::MoveTo(position) => {
                                                action
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
                                        action.send(PlayerAction::disconnect(id)).await.unwrap();
                                    }
                                    _ => print!("got a unhandable error"),
                                }
                            }
                        }
                    }
                });
                Ok(())
            }
        }
        
    }
}
