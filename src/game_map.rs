use crate::vector2::{Vector2, Vector2Range};

pub enum TileType {
    Walkable(char),
    Wall(char),
    OutOfBounds(char)
}

impl TileType {
    pub fn from_char (tile : char) -> Self {
        match tile {
            '#' => Self::Wall(tile),
            '.' => Self::Walkable(tile),
            _ => Self::OutOfBounds(tile)
        }
    }
}

impl std::fmt::Display for TileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Walkable(char) => write!(f, "Walkable: {}", char),
            Self::Wall(char) => write!(f, "Wall: {}", char),
            Self::OutOfBounds(char) => write!(f, "OutOfBounds: {}", char),
        }
    }
}

type WalkMap = std::collections::HashMap<Vector2, TileType>;

pub struct GameMap{
    pub string_map: String,
    width: i32,
    walk_map: WalkMap
}

pub enum WalkResult {
    Finished(Vector2),
    Unfinished{
        hit: Vector2,
        safe: Vector2
    }
}

impl GameMap {
    pub fn from_file_path(file_path: &str, width: i32) -> Self {
        let string_map = std::fs::read_to_string(&file_path).expect("Should be able to open the map file");
        let mut walk_map = WalkMap::new();
        for (y, line) in string_map.lines().enumerate() {
            for (x, char) in line.chars().enumerate() {
                walk_map.entry(Vector2(x as i32, y as i32)).or_insert(TileType::from_char(char));
            }
        }
        Self {
            string_map: string_map,
            width,
            walk_map
        }
    }

    pub fn on_position(&self, position: &Vector2) -> &TileType {
        self.walk_map
            .get(position)
            .unwrap_or(&TileType::OutOfBounds('0'))
    }

    pub fn cast(&self, from: &Vector2, to: &Vector2) -> WalkResult {
        let mut last = from.clone();
        for position in Vector2Range::new(from, to) {
            if let TileType::Wall(_) | TileType::OutOfBounds(_) = self.on_position(&position) {
                return WalkResult::Unfinished{
                    hit: position,
                    safe: last
                };
            }
            last = position;
        }
        WalkResult::Finished(to.clone())
    }

    pub fn plot(&mut self, from: &Vector2, to: &Vector2, to_place: char) {
        for position in Vector2Range::new(from, to) {
            println!("plotting at {}", &position);
            self.set_char(&position, to_place);
        }
    }

    pub fn set_char(&mut self, position: &Vector2, to_place: char) {
        let index = (((self.width + 2) * position.1) + position.0) as usize;
        self.string_map.replace_range(index..index+1, to_place.to_string().get(..1).unwrap())
    }
}