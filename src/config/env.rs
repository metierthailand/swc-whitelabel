use anyhow::{Context, Result};
use serde::Deserialize;
use std::{cell::RefCell, env, fs, path::PathBuf};

// 1. Define the struct that perfectly mirrors your JSON
#[derive(Debug, Deserialize, Clone)]
pub struct WhitelabelConfig {
    pub src: String,
    pub patterns: Vec<String>,
    pub output_dir: String,
    pub default_target: String,
    #[serde(default = "tsconfig")]
    pub tsconfig: String,
    #[serde(skip)]
    pub output_file_name_only: bool,
    #[serde(skip)]
    pub cwd: PathBuf,
}

fn tsconfig() -> String {
    "tsconfig.json".to_string()
}

thread_local! {
    static CONFIG: RefCell<Option<WhitelabelConfig>> = const { RefCell::new(None) };
}

pub fn init(cwd: Option<PathBuf>, config_filename: &str) -> Result<()> {
    let resolved_cwd = cwd.map_or_else(env::current_dir, Ok)?;
    let mut resolved_file = resolved_cwd.clone();

    resolved_file.push(config_filename);

    // Read the file to a string
    let config_str = fs::read_to_string(&resolved_file).context(format!(
        "Failed to read config file at {}",
        resolved_file.display()
    ))?;

    println!("{}", resolved_file.to_string_lossy());

    // Deserialize the JSON string into our struct
    let mut config: WhitelabelConfig = serde_json::from_str(&config_str)
        .context("Failed to parse whitelabel.config.json. Is the JSON strictly valid?")?;

    let mut resolved_tsconfig_path = resolved_cwd.clone();
    resolved_tsconfig_path.push(config.tsconfig);

    config.output_file_name_only = std::env::args().any(|arg| arg == "--file-name-only");
    config.cwd = resolved_cwd;

    config.tsconfig = resolved_tsconfig_path.to_string_lossy().to_string();

    // Lock the config into our global state. It can never be overwritten!
    CONFIG.with(|c| {
        *c.borrow_mut() = Some(config);
    });

    Ok(())
}

pub fn with_config<F, R>(f: F) -> R
where
    F: FnOnce(&WhitelabelConfig) -> R,
{
    CONFIG.with(|c| {
        let borrow = c.borrow();
        let cfg = borrow
            .as_ref()
            .expect("FATAL: Config not initialized on this thread!");
        f(cfg)
    })
}
