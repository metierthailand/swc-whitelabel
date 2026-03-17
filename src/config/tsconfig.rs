use anyhow::{Context, Result};
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

// 2. Helper function to read and parse the config
pub fn load(tsconfig: String) -> Result<TsConfig> {
    // Read the file to a string
    let config_str = fs::read_to_string(&tsconfig)
        .context(format!("Failed to read config file at {}", &tsconfig))?;

    // Deserialize the JSON string into our struct
    let config: TsConfig = serde_json::from_str(&config_str).context(format!(
        "Failed to parse {}. Is the JSON strictly valid?",
        &tsconfig
    ))?;

    Ok(config)
}
