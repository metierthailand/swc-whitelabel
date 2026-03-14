use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;

// 1. Define the struct that perfectly mirrors your JSON
#[derive(Debug, Deserialize)]
pub struct WhitelabelConfig {
    pub src: String,
    pub patterns: Vec<String>,
    pub output_dir: String,
    pub default_target: String,
}

// 2. Helper function to read and parse the config
pub fn load_config() -> Result<WhitelabelConfig> {
    let config_path = "whitelabel.config.json";

    // Read the file to a string
    let config_str = fs::read_to_string(config_path)
        .context(format!("Failed to read config file at {}", config_path))?;

    // Deserialize the JSON string into our struct
    let config: WhitelabelConfig = serde_json::from_str(&config_str)
        .context("Failed to parse whitelabel.config.json. Is the JSON strictly valid?")?;

    Ok(config)
}
