use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_database_path")]
    pub database_path: String,
    #[serde(default = "default_mqtt_device_name")]
    pub mqtt_device_name: String,
    #[serde(default = "default_mqtt_host")]
    pub mqtt_host: String,
}

fn default_database_path() -> String {
    "local.db".to_string()
}

fn default_mqtt_device_name() -> String {
    "weather-station-backend".to_string()
}

fn default_mqtt_host() -> String {
    "localhost".to_string()
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let text = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&text)?;
        Ok(config)
    }
}
