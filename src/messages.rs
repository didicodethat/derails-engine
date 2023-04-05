use serde::{Serialize, Deserialize};
use serde_json;
use schemars::{JsonSchema, schema_for};
use std::{fs::File, io::Write};
use crate::vector2::Vector2;


pub trait SimpleJSON<'a> {
    type A: Serialize + Deserialize<'a>;
    fn to_json(owner: &Self::A) -> Result<String, serde_json::Error> {
        serde_json::to_string(owner)
    }
    fn from_json(str : &'a str) -> Result<Self::A, serde_json::Error> {
        serde_json::from_str::<Self::A>(str)
    }
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub enum ServerMessages {
    PlayerPositionUpdate(u32, Vector2),
    MapDisplay(String),
    BadMessageFormatting,
}

impl<'a> SimpleJSON<'a> for ServerMessages {
    type A = Self;
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub enum ClientMessages {
    MoveTo(Vector2)
}

impl<'a> SimpleJSON<'a> for ClientMessages {
    type A = Self;
}

pub fn write_schemas_to_files() {
    let mut server_message_file = File::create("schemas/server_messages.json").unwrap();
    let mut client_message_file = File::create("schemas/client_messages.json").unwrap();
    let server_messages_schema = schema_for!(ServerMessages);
    let client_messages_schema = schema_for!(ClientMessages);
    server_message_file.write_all( serde_json::to_string_pretty(&server_messages_schema).unwrap().as_bytes()).unwrap();
    client_message_file.write_all( serde_json::to_string_pretty(&client_messages_schema).unwrap().as_bytes()).unwrap();
}