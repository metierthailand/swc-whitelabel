use anyhow::{Context, Result};
use serde::Deserialize;
use std::{env, fs, path::PathBuf, sync::OnceLock};

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
    #[serde(skip)]
    pub cwd: PathBuf,
}

pub static CONFIG: OnceLock<WhitelabelConfig> = OnceLock::new();

pub fn get() -> &'static WhitelabelConfig {
    CONFIG
        .get()
        .expect("FATAL: Tried to read config before it was initialized!")
}

pub fn init(cwd: Option<PathBuf>, config_filename: &str) -> Result<()> {
    let resolved_cwd = cwd.unwrap_or(env::current_dir().unwrap());
    let mut resolved_file = resolved_cwd.clone();

    resolved_file.push(config_filename);

    // Read the file to a string
    let config_str = fs::read_to_string(&resolved_file).context(format!(
        "Failed to read config file at {}",
        resolved_file.display()
    ))?;

    // Deserialize the JSON string into our struct
    let mut config: WhitelabelConfig = serde_json::from_str(&config_str)
        .context("Failed to parse whitelabel.config.json. Is the JSON strictly valid?")?;

    let mut resolved_tsconfig_path = resolved_cwd.clone();
    resolved_tsconfig_path.push(config.tsconfig.unwrap_or("tsconfig.json".into()));

    config.output_file_name_only = std::env::args().any(|arg| arg == "--file-name-only");
    config.cwd = resolved_cwd;

    config.tsconfig = Some(resolved_tsconfig_path.to_string_lossy().to_string());

    // Lock the config into our global state. It can never be overwritten!
    CONFIG
        .set(config)
        .map_err(|_| anyhow::anyhow!("Config was already initialized!"))?;

    Ok(())
}
