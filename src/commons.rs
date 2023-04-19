use serde::Deserialize;
use std::fs::read_to_string;

#[derive(Deserialize)]
pub struct Settings {
    pub game: GameSettings,
}

#[derive(Deserialize)]
pub struct GameSettings {
    pub address: String,
    pub port: i64,
}

pub fn get_settings() -> Settings {
    let settings_str = read_to_string("settings.toml").expect("Couldn't access settings.toml");
    toml::from_str(&settings_str).expect("Couldn't parse the settings file")
}
