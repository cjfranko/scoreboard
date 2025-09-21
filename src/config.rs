use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub scoreboard: ScoreboardConfig,
    pub rugby: RugbyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub web_port: u16,
    pub simulation_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreboardConfig {
    pub address: String,
    pub card_id: u8,
    pub connection_timeout_seconds: u64,
    pub reconnect_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugbyConfig {
    pub try_points: u16,
    pub conversion_points: u16,
    pub penalty_points: u16,
    pub first_half_minutes: u8,
    pub second_half_minutes: u8,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                web_port: 3030,
                simulation_mode: false,
            },
            scoreboard: ScoreboardConfig {
                address: "192.168.1.100:5200".to_string(),
                card_id: 1,
                connection_timeout_seconds: 5,
                reconnect_interval_seconds: 30,
            },
            rugby: RugbyConfig {
                try_points: 5,
                conversion_points: 2,
                penalty_points: 3,
                first_half_minutes: 40,
                second_half_minutes: 40,
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = "config.yaml";
        
        if Path::new(config_path).exists() {
            let config_content = fs::read_to_string(config_path)?;
            let config: Config = serde_yaml::from_str(&config_content)?;
            log::info!("Loaded configuration from {}", config_path);
            Ok(config)
        } else {
            let default_config = Config::default();
            default_config.save()?;
            log::info!("Created default configuration file at {}", config_path);
            Ok(default_config)
        }
    }
    
    pub fn save(&self) -> Result<()> {
        let config_content = serde_yaml::to_string(self)?;
        fs::write("config.yaml", config_content)?;
        Ok(())
    }
    
    pub fn reload(&mut self) -> Result<()> {
        let new_config = Config::load()?;
        *self = new_config;
        Ok(())
    }
}