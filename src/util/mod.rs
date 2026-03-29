use crate::config::env::{self, WhitelabelConfig};
use pathdiff::diff_paths;
use std::path::{Path, PathBuf};

pub fn report<F>(f: F)
where
    F: FnOnce(),
{
    env::with_config(|cfg| {
        if !cfg.output_file_name_only {
            f()
        };
    })
}

pub fn create_reporter<F>(pred: F) -> impl Fn(Box<dyn FnOnce()>)
where
    F: FnOnce(&WhitelabelConfig) -> bool,
{
    let is_allowed = env::with_config(|cfg| pred(cfg));

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

pub fn to_rel_import(current_dir: &Path, import_path: &str) -> PathBuf {
    let absolute_target = env::with_config(|cfg| cfg.cwd.join(&cfg.src).join(import_path));

    match compute_relative_import(current_dir, &absolute_target) {
        Some(s) => PathBuf::from(s).with_extension(""),
        None => PathBuf::from(import_path), // Safe fallback
    }
}

pub fn cname(input: PathBuf) -> Option<PathBuf> {
    let Some(file_name) = input.file_name() else {
        return None;
    };
    let clean_dir = input.parent().and_then(|dir| dir.canonicalize().ok());

    clean_dir.map(|mut p| {
        p.push(file_name);
        p.with_extension("")
    })
}

pub mod resolver;
pub mod transactional;
