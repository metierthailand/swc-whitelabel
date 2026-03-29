use anyhow::{Error, Ok, anyhow};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::env;
use crate::util::cname;

#[derive(Clone, Debug)]
pub struct TsImportPathResolver {
    pub path_mapping: HashMap<String, String>,
}

impl TsImportPathResolver {
    // TODO: `baseUrl`
    pub fn resolve_import(&self, current_file_path: PathBuf, import_src: &str) -> Option<PathBuf> {
        let cwd = env::with_config(|cfg| cfg.cwd.clone());

        // 🎯 CATEGORY 1: Relative Import
        if import_src.starts_with('.')
            && let Some(parent) = current_file_path.parent()
        {
            return cname(parent.join(import_src).as_path());
        }
        /* 🎯 CATEGORY 2: TSConfig Path Resolution */
        // Step 1: Check for an EXACT match (e.g., "@/app/whitelabel")
        else if let Some(mapped_path) = self.path_mapping.get(import_src) {
            // Only first in record
            return cname(cwd.join(mapped_path).as_path());
        }
        // Step 2: Check for a WILDCARD match (e.g., "@app/*")
        else if let Some((pattern, mapped_path, _)) = self.best_path_mapping_match(import_src)
            && let Some(star_idx) = pattern.find('*')
        {
            let prefix = &pattern[..star_idx];
            let suffix = &pattern[star_idx + 1..];

            // Extract the string that replaces the '*'
            let wildcard_match = &import_src[prefix.len()..import_src.len() - suffix.len()];

            let resolved_mapped = mapped_path.replace("*", wildcard_match);

            return cname(cwd.join(resolved_mapped).as_path());
        }

        // TODO node_modules / turbo repo
        None
    }

    fn best_path_mapping_match(&self, import_src: &str) -> Option<(&String, &String, usize)> {
        let mut best_match: Option<(&String, &String, usize)> = None;
        for (pattern, mapped_path) in &self.path_mapping {
            if let Some(star_idx) = pattern.find('*') {
                let prefix = &pattern[..star_idx];
                let suffix = &pattern[star_idx + 1..];

                if import_src.starts_with(prefix) && import_src.ends_with(suffix) {
                    let match_len = prefix.len() + suffix.len();
                    // TypeScript Rule: Longest prefix match wins!
                    if best_match.is_none_or(|(_, _, size)| match_len > size) {
                        best_match = Some((pattern, mapped_path, match_len));
                    }
                }
            }
        }

        best_match
    }
}

impl TryFrom<HashMap<String, Vec<String>>> for TsImportPathResolver {
    type Error = Error;

    fn try_from(value: HashMap<String, Vec<String>>) -> Result<Self, Self::Error> {
        let path_mapping_result: Result<HashMap<String, String>, _> = value
            .into_iter()
            .map(|(key, mut vec)| {
                if vec.len() != 1 {
                    return Err(anyhow!(
                        "Multiple aliases ({}) = {} are not supported yet",
                        key,
                        vec.len()
                    ));
                }

                // Since we just verified len == 1, pop() is 100% safe.
                // (We use pop() because it takes ownership without allocating).
                Ok((key, vec.pop().unwrap()))
            })
            // This magically transposes Iterator<Result> -> Result<HashMap>
            .collect();

        match path_mapping_result {
            std::result::Result::Ok(path_mapping) => Ok(Self { path_mapping }),
            Err(e) => Err(anyhow!(e)),
        }
    }
}
