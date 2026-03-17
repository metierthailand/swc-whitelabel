use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, sync::OnceLock};

// 1. Define the struct that perfectly mirrors your JSON
#[derive(Debug, Deserialize)]
pub struct WhitelabelConfig {
    pub src: String,
    pub patterns: Vec<String>,
    pub output_dir: String,
    pub default_target: String,
    pub tsconfig: Option<String>,
    #[serde(skip)]
    pub output_file_name_only: bool,
}

pub static CONFIG: OnceLock<WhitelabelConfig> = OnceLock::new();

pub fn get() -> &'static WhitelabelConfig {
    CONFIG
        .get()
        .expect("FATAL: Tried to read config before it was initialized!")
}

pub fn init() -> Result<()> {
    let config_path = "whitelabel.config.json";

    // Read the file to a string
    let config_str = fs::read_to_string(config_path)
        .context(format!("Failed to read config file at {}", config_path))?;

    // Deserialize the JSON string into our struct
    let mut config: WhitelabelConfig = serde_json::from_str(&config_str)
        .context("Failed to parse whitelabel.config.json. Is the JSON strictly valid?")?;

    config.output_file_name_only = std::env::args().any(|arg| arg == "--file-name-only");

    // Lock the config into our global state. It can never be overwritten!
    CONFIG
        .set(config)
        .map_err(|_| anyhow::anyhow!("Config was already initialized!"))?;

    Ok(())
}
