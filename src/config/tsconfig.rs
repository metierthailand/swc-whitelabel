use anyhow::Result;
use serde::Deserialize;
use std::{collections::HashMap, fs};

#[derive(Debug, Deserialize, Clone)]
pub struct CompilerOptions {
    pub paths: HashMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TsConfig {
    #[serde(rename = "compilerOptions")]
    pub compiler_options: CompilerOptions,
}

impl Default for TsConfig {
    fn default() -> Self {
        Self {
            compiler_options: CompilerOptions {
                paths: HashMap::new(),
            },
        }
    }
}

// 2. Helper function to read and parse the config
pub fn load(tsconfig: String) -> Result<TsConfig> {
    let config: TsConfig = {
        if let Ok(config_str) = fs::read_to_string(&tsconfig)
            && let Ok(cfg) = serde_json::from_str(&config_str)
        {
            cfg
        } else {
            Default::default()
        }
    };

    Ok(config)
}
