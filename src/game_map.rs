use crate::{
    messages::{PlayerBroadcastAction, ServerMessages, SimpleJSON},
    player::{PlayerAction, PlayerConnection, PlayerConnectionId},
    vector2::{Vector2, Vector2Range},
};
use futures_util::SinkExt;
use std::{
    collections::{HashMap, VecDeque},
    thread::spawn,
};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
type GameMapId = usize;
const CONNECTION_QUEUE_SIZE: usize = 32;
const ACTION_QUEUE_SIZE: usize = 32;

pub enum TileType {
    Walkable(char),
    Wall(char),
    OutOfBounds(char),
    SpawnPosition(char),
}

impl TileType {
    pub fn from_char(tile: char) -> Self {
        match tile {
            '#' => Self::Wall(tile),
            '.' => Self::Walkable(tile),
            '@' => Self::SpawnPosition(tile),
            _ => Self::OutOfBounds(tile),
        }
    }
}

type WalkMap = std::collections::HashMap<Vector2, TileType>;

pub enum WalkResult {
    Finished(Vector2),
    Unfinished { hit: Vector2, safe: Vector2 },
}

pub struct GameMap {
    pub string_map: String,
    width: i32,
    walk_map: WalkMap,
}

impl GameMap {
    pub fn from_file_path(file_path: &str, width: i32) -> Self {
        let string_map =
            std::fs::read_to_string(file_path).expect("Should be able to open the map file");
        let mut walk_map = WalkMap::new();
        for (y, line) in string_map.lines().enumerate() {
            for (x, char) in line.chars().enumerate() {
                walk_map
                    .entry(Vector2(x as i32, y as i32))
                    .or_insert(TileType::from_char(char));
            }
        }

        Self {
            string_map,
            width,
            walk_map,
        }
    }

    pub fn spawn_position(&self) -> Option<Vector2> {
        for (position, tile) in &self.walk_map {
            if let TileType::SpawnPosition(_) = tile {
                return Some(*position);
            }
        }
        None
    }

    pub fn on_position(&self, position: &Vector2) -> &TileType {
        self.walk_map
            .get(position)
            .unwrap_or(&TileType::OutOfBounds('0'))
    }

    pub fn cast(&self, from: &Vector2, to: &Vector2) -> WalkResult {
        let mut last = *from;
        for position in Vector2Range::new(from, to) {
            if let TileType::Wall(_) | TileType::OutOfBounds(_) = self.on_position(&position) {
                return WalkResult::Unfinished {
                    hit: position,
                    safe: last,
                };
            }
            last = position;
        }
        WalkResult::Finished(*to)
    }

    pub fn plot(&mut self, from: &Vector2, to: &Vector2, to_place: char) {
        for position in Vector2Range::new(from, to) {
            self.set_char(&position, to_place);
        }
    }

    pub fn set_char(&mut self, position: &Vector2, to_place: char) {
        let index = (((self.width + 2) * position.1) + position.0) as usize;
        self.string_map
            .replace_range(index..index + 1, to_place.to_string().get(..1).unwrap())
    }
}

pub struct GameMapServer {
    id: GameMapId,
    map: GameMap,
    players: HashMap<PlayerConnectionId, PlayerConnection>,
    messages: VecDeque<ServerMessages>,
    raw_messages: VecDeque<String>,
    dead_connections: VecDeque<PlayerConnectionId>,
    connection_receiver: Receiver<PlayerConnection>,
    action_receiver: Receiver<PlayerAction>,
    pub connection_funnel: Sender<PlayerConnection>,
    pub action_funnel: Sender<PlayerAction>,
}

impl GameMapServer {
    pub fn new(id: GameMapId, map: GameMap) -> Self {
        let (action_funnel, action_receiver) = channel::<PlayerAction>(ACTION_QUEUE_SIZE);
        let (connection_funnel, connection_receiver) =
            channel::<PlayerConnection>(CONNECTION_QUEUE_SIZE);
        Self {
            id,
            map,
            players: HashMap::new(),
            messages: VecDeque::new(),
            raw_messages: VecDeque::new(),
            dead_connections: VecDeque::new(),
            connection_funnel,
            connection_receiver,
            action_funnel,
            action_receiver,
        }
    }

    pub fn spawn_position(&self) -> Vector2 {
        self.map.spawn_position().unwrap_or(Vector2(0, 0))
    }

    pub async fn step(&mut self) {
        while let Ok(mut player_connection) = self.connection_receiver.try_recv() {
            self.messages.push_back(ServerMessages::PlayerConnected(
                player_connection.id,
                self.spawn_position(),
            ));
            player_connection.player.position = self.spawn_position();
            player_connection.read_messages(&self.action_funnel).unwrap();
            self.players.insert(player_connection.id, player_connection);
        }

        while let Ok(action) = self.action_receiver.try_recv() {
            let entry = self.players.get_mut(&action.id);
            if let Some(player_connection) = entry {
                player_connection
                    .player
                    .update_action(action.action, Some(&self.map));
            }
        }
        self.execute_player_actions();
        self.broadcast_messages().await;
        self.disconnect_dead_connections().await;
    }

    fn execute_player_actions(&mut self) {
        for (id, connection) in self.players.iter_mut() {
            if let Some(action) = connection.player.next() {
                if let PlayerBroadcastAction::None = action {
                    continue;
                }
                self.messages
                    .push_back(ServerMessages::BroadCastAction(*id, action));
            } else {
                self.dead_connections.push_back(*id);
            }
        }
    }

    async fn broadcast_messages(&mut self) {
        while let Some(respose) = self.messages.pop_front() {
            for (id, mut connection) in self.players.iter_mut() {
                let result = connection
                    .sink
                    .send(Message::Text(ServerMessages::to_json(&respose).unwrap()))
                    .await;

                if let Err(result) = result {
                    match result {
                        tokio_tungstenite::tungstenite::Error::ConnectionClosed
                        | tokio_tungstenite::tungstenite::Error::AlreadyClosed => {
                            self.dead_connections.push_back(*id);
                        }
                        _ => {
                            todo!(
                                "couldn't send a answer to a user, but connection isn't dead yet"
                            );
                        }
                    }
                }
            }
        }
    }

    async fn disconnect_dead_connections(&mut self) {
        let mut disconnect_messages = Vec::new();

        for id in &self.dead_connections {
            self.players.remove(id);
        }

        while let Some(id) = self.dead_connections.pop_front() {
            let message = ServerMessages::PlayerDisconnected(id);
            if let Ok(string) = ServerMessages::to_json(&message) {
                disconnect_messages.push(string);
            }
        }

        for message in &disconnect_messages {
            for (_, connection) in self.players.iter_mut() {
                // Don't care if this message fails, we handle them next time.
                let _ = connection.sink.send(Message::text(message)).await;
            }
        }
    }
}
