use anyhow::Result;
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

impl Default for WhitelabelConfig {
    fn default() -> Self {
        Self {
            src: "app/".to_owned(),
            patterns: vec!["**/*.tsx".to_owned(), "**/*.ts".to_owned()],
            output_dir: "whitelabel".to_owned(),
            default_target: "def".to_owned(),
            tsconfig: "tsconfig.json".to_owned(),
            output_file_name_only: Default::default(),
            cwd: Default::default(),
        }
    }
}

fn tsconfig() -> String {
    "tsconfig.json".to_string()
}

thread_local! {
    static CONFIG: RefCell<Option<WhitelabelConfig>> = const { RefCell::new(None) };
}

pub fn init(cwd: Option<PathBuf>, config_filename: &str) -> Result<()> {
    let resolved_cwd = cwd.map_or_else(env::current_dir, Ok)?;
    let mut config: WhitelabelConfig = {
        let mut resolved_file = resolved_cwd.clone();

        resolved_file.push(config_filename);

        // Read the file to a string
        if let Ok(config_str) = fs::read_to_string(&resolved_file)
            && let Ok(config_from_json) = serde_json::from_str::<WhitelabelConfig>(&config_str)
        {
            config_from_json
        } else {
            Default::default()
        }
    };

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
