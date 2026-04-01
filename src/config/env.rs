use anyhow::Result;
use serde::Deserialize;
use std::{cell::RefCell, env, fs, path::PathBuf};

fn default_tsconfig() -> String {
    "tsconfig.json".to_string()
}

fn default_patterns() -> Vec<String> {
    vec!["**/*.tsx".to_owned(), "**/*.ts".to_owned()]
}

fn default_output_dir() -> String {
    "whitelabel".to_string()
}

// 1. Define the struct that perfectly mirrors your JSON
#[derive(Debug, Deserialize, Clone)]
pub struct WhitelabelConfig {
    pub src: String,
    pub default_target: String,
    #[serde(default = "default_patterns")]
    pub patterns: Vec<String>,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default = "default_tsconfig")]
    pub tsconfig: String,
    #[serde(skip)]
    pub output_file_name_only: bool,
    #[serde(skip)]
    pub with_manifest: bool,
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
            with_manifest: Default::default(),
            cwd: Default::default(),
        }
    }
}

thread_local! {
    static CONFIG: RefCell<Option<WhitelabelConfig>> = const { RefCell::new(None) };
}

pub fn init(config: WhitelabelConfig) -> Result<()> {
    // Lock the provided config into our global state.
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
