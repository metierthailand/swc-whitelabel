use anyhow::{Result, anyhow};
use std::collections::HashSet;
use std::{collections::HashMap, path::PathBuf};

use crate::ast::collector::{WhitelabelCollector, WhitelabelEntry, WhitelabelTarget};
use crate::config::env::with_config;
use crate::util::report;

pub fn try_build_registry_maps(
    collector: &WhitelabelCollector<'_>,
) -> Result<HashMap<String, Vec<WhitelabelEntry>>> {
    let mut seen_keys: HashSet<(String, String)> = HashSet::new();
    let mut grouped_entries: HashMap<String, Vec<WhitelabelEntry>> = HashMap::new();

    let root_dir = with_config(|cfg| cfg.cwd.join(&cfg.src));

    let mut cloned = collector.entries.clone();

    let mut targetted = cloned.iter_mut().filter(|p| match p.target {
        WhitelabelTarget::Targetted(_) => true,
        WhitelabelTarget::Wildcard => false,
    });

    targetted.try_for_each(|entry| {
        let WhitelabelTarget::Targetted(target) = entry.target.clone() else {
            return Err(anyhow!("Unexpected"));
        };

        let pb = PathBuf::from(&entry.import_path);

        // Safely strip the absolute project root to guarantee a relative snapshot path
        let relative_pb = pb.strip_prefix(root_dir.clone()).unwrap_or(&pb);

        entry.import_path = relative_pb.to_string_lossy().to_string();

        report(|| {
            println!(
                "\t🪡 ({}) found {} @ {}",
                target, entry.symbol, entry.import_path
            );
        });

        let unique_key = (target.clone(), entry.key.clone());
        if seen_keys.contains(&unique_key) {
            return Err(anyhow!("Error: duplicate key found {:?}", unique_key));
        }

        seen_keys.insert(unique_key);

        grouped_entries
            .entry(target.clone())
            .or_default()
            .push(entry.clone());

        Ok(())
    })?;

    Ok(grouped_entries)
}
