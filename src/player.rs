use tokio::net::TcpStream;

use crate::{vector2::{Vector2, Vector2Range}, messages::PlayerBroadcastAction};

type PlayerSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<TcpStream>,
    tokio_tungstenite::tungstenite::Message,
>;

#[derive(Copy, Clone, Debug)]
pub enum PlayerActions {
    MoveTo(Vector2),
    Idle,
    Disconnect,
}

#[derive(Copy, Clone, Debug)]
pub struct PlayerAction {
    pub id: u32,
    pub action: PlayerActions,
}

impl PlayerAction {
    pub fn disconnect(id: u32) -> Self {
        Self {
            id,
            action: PlayerActions::Disconnect,
        }
    }

    pub fn move_to(id:u32, position : Vector2) -> Self {
        Self {
            id,
            action: PlayerActions::MoveTo(position),
        }
    }
}

#[derive(Debug)]
pub struct PlayerConnected {
    pub id: u32,
    pub sink: PlayerSink,
}

impl PlayerConnected {
    pub fn new (id: u32, sink: PlayerSink) -> Self {
        Self {id, sink}
    }
}

pub struct Player {
    pub position: Vector2,
    pub current_action: PlayerActions,
    pub current_path: Option<Vector2Range>,
}

impl Player {
    pub fn new(position: Vector2) -> Self {
        return Self {
            position,
            current_action: PlayerActions::Idle,
            current_path: None,
        };
    }

    pub fn update_action(&mut self, action: PlayerActions) {
        self.current_action = action;
        if let PlayerActions::MoveTo(target_position) = action {
            self.current_path = Some(Vector2Range::new(&self.position, &target_position));
        }
    }
}

impl Iterator for Player {
    type Item = PlayerBroadcastAction;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current_action {
            PlayerActions::MoveTo(_) => match self.current_path {
                None => {
                    self.update_action(PlayerActions::Idle);
                    Some(PlayerBroadcastAction::None)
                }
                Some(mut path) => match path.next() {
                    Some(step) => {
                        self.position = step;
                        self.current_path = Some(path);
                        Some(PlayerBroadcastAction::Step(step))
                    }
                    None => {
                        self.update_action(PlayerActions::Idle);
                        Some(PlayerBroadcastAction::None)
                    }
                },
            },
            PlayerActions::Idle => Some(PlayerBroadcastAction::None),
            PlayerActions::Disconnect => None,
        }
    }
}

pub struct PlayerConnection {
    pub sink: PlayerSink,
    pub player: Player,
}

impl PlayerConnection {
    pub fn new(sink: PlayerSink) -> Self {
        Self {
            sink,
            player: Player::new(Vector2(0, 0)),
        }
    }
}