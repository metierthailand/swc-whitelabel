use std::{fs, path::PathBuf, process};

use anyhow::Result;
use wl_extractor::{
    config::env::WhitelabelConfig,
    run::{RunOptions, run},
};

struct DefaultRunOptions {
    pub cwd: Option<PathBuf>,
    pub config_filename: String,
}

impl Default for DefaultRunOptions {
    fn default() -> Self {
        Self {
            cwd: None,
            config_filename: "whitelabel.config.json".to_string(),
        }
    }
}

impl RunOptions for DefaultRunOptions {
    fn provide_config(&self) -> Result<WhitelabelConfig> {
        let resolved_cwd = self.cwd.clone().map_or_else(std::env::current_dir, Ok)?;
        let resolved_file = resolved_cwd.join(&self.config_filename);

        // Read and deserialize, or fallback to default
        let mut config = if let Ok(config_str) = fs::read_to_string(&resolved_file) {
            serde_json::from_str::<WhitelabelConfig>(&config_str)?
        } else {
            WhitelabelConfig::default()
        };

        let output_file_name_only = std::env::args().any(|arg| arg == "--file-name-only");
        let with_manifest = std::env::args().any(|arg| arg == "--with-manifest");

        // Hydrate the runtime-only fields
        config.cwd = resolved_cwd.clone();
        config.output_file_name_only = output_file_name_only;
        config.with_manifest = with_manifest;
        config.tsconfig = resolved_cwd
            .join(&config.tsconfig)
            .to_string_lossy()
            .to_string();

        Ok(config)
    }
}

fn main() {
    match run(DefaultRunOptions::default()) {
        Ok(_) => process::exit(0),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            process::exit(1)
        }
    }
}
