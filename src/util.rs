use crate::config::config::{self, WhitelabelConfig};
use pathdiff::diff_paths;
use std::path::Path;

pub fn report<F>(f: F)
where
    F: FnOnce(),
{
    config::with_config(|cfg| {
        if !cfg.output_file_name_only {
            f()
        };
    })
}

pub fn create_reporter<F>(pred: F) -> impl Fn(Box<dyn FnOnce()>)
where
    F: FnOnce(&WhitelabelConfig) -> bool,
{
    let is_allowed = config::with_config(|cfg| pred(cfg));

    move |f| {
        if is_allowed {
            f()
        }
    }
}

/// Computes a JS-compatible relative import path
pub fn compute_relative_import(current_file_dir: &Path, resolved_target: &Path) -> Option<String> {
    // 1. Calculate the raw relative path
    let relative_path = diff_paths(resolved_target, current_file_dir)?;

    // 2. Convert to string and enforce forward slashes (crucial for JS on Windows)
    let mut relative_str = relative_path.to_string_lossy().replace('\\', "/");

    // 3. JS module resolution requires relative paths to start with `./` or `../`
    // If diff_paths returns "app/whitelabel", JS will think it's an npm package.
    // We must prefix it with "./" to make it explicitly relative.
    if !relative_str.starts_with('.') && !relative_str.starts_with('/') {
        relative_str.insert_str(0, "./");
    }

    Some(relative_str)
}
