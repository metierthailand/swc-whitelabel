use anyhow::{Error, anyhow};
use std::collections::HashSet;
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

use crate::ast::collector::{WhitelabelEntry, WhitelabelTarget};
use crate::config::env::{self, with_config};
use crate::util::{cname, report};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum WhitelabelSymbol {
    Symbol { symbol: String, import_path: String },
    Undefined,
}

impl WhitelabelSymbol {
    pub fn short_id(&self) -> u64 {
        let mut hasher = DefaultHasher::new();

        // Because we #[derive(Hash)], this safely hashes the entire variant
        // (including the enum discriminant, symbol, and import_path)
        self.hash(&mut hasher);

        hasher.finish()
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct WhitelabelRecord {
    pub target: String,
    pub key: String,
    pub symbol: WhitelabelSymbol,
    pub remark: String,
}

type Target = String;
type Key = String;

#[derive(Clone)]
pub struct WhitelabelRegistry {
    table: HashMap<Target, HashMap<Key, WhitelabelRecord>>,
    pivoted: HashMap<Key, HashSet<Target>>,
}

impl WhitelabelRegistry {
    pub fn by_keys(&self) -> Vec<(&String, Vec<&WhitelabelRecord>)> {
        self.pivoted
            .iter()
            .map(|(key, targets)| {
                let records = targets
                    .iter()
                    .filter_map(|t| self.table.get(t)?.get(key))
                    .collect::<Vec<_>>();
                (key, records)
            })
            .collect()
    }

    pub fn targets(&self) -> Vec<&String> {
        self.table.keys().collect()
    }

    pub fn get_target_entries(&self, target: &String) -> Vec<&WhitelabelRecord> {
        self.table
            .get(target)
            .map(|r| r.values().collect::<Vec<_>>())
            .unwrap_or_default()
    }

    pub fn lookup(&self, name: &String, abs_resolved_path: &Path) -> Option<WhitelabelEntry> {
        let entries = self.pivoted.get(name).map(|targets| {
            targets
                .iter()
                .filter_map(|t| self.table.get(t)?.get(name))
                .collect::<Vec<_>>()
        })?;

        let import_match = entries.iter().find(|entry| {
            let WhitelabelSymbol::Symbol {
                symbol: _,
                import_path,
            } = &entry.symbol
            else {
                return false;
            };

            let Some(absolute_import_path) =
                env::with_config(|cfg| cname(cfg.cwd.join(&cfg.src).join(import_path).as_path()))
            else {
                return false;
            };

            let match_exact = absolute_import_path == abs_resolved_path;

            let match_parent = absolute_import_path.parent() == Some(abs_resolved_path);

            match_exact || match_parent
        });

        match import_match {
            Some(e) => match &e.symbol {
                WhitelabelSymbol::Symbol {
                    symbol,
                    import_path,
                } => Some(WhitelabelEntry {
                    target: WhitelabelTarget::Targetted(e.target.to_owned()),
                    key: e.key.to_owned(),
                    symbol: symbol.to_owned(),
                    import_path: import_path.to_owned(),
                    _experiment_remark: e.remark.to_owned(),
                    optional: false,
                }),
                WhitelabelSymbol::Undefined => None,
            },
            None => None,
        }
    }
}

impl TryFrom<Vec<WhitelabelEntry>> for WhitelabelRegistry {
    type Error = Error;

    fn try_from(entries: Vec<WhitelabelEntry>) -> std::result::Result<Self, Self::Error> {
        let mut grouped_entries: HashMap<Target, HashMap<Key, WhitelabelRecord>> = HashMap::new();

        let root_dir = with_config(|cfg| cfg.cwd.join(&cfg.src));

        let (optional, implemented): (Vec<_>, Vec<_>) =
            entries.into_iter().partition(|p| p.optional);

        let (wildcards, targetted): (Vec<_>, Vec<_>) = implemented
            .into_iter()
            .partition(|p| matches!(p.target, WhitelabelTarget::Wildcard));

        for entry in targetted {
            let target = match entry.target {
                WhitelabelTarget::Targetted(t) => t,
                other => return Err(anyhow!("Unexpected wildcard: {}", other)),
            };

            let pb = PathBuf::from(entry.import_path);

            // Safely strip the absolute project root to guarantee a relative snapshot path
            let relative_pb = pb.strip_prefix(&root_dir).unwrap_or(&pb);

            let rel_import_path = relative_pb.to_string_lossy().to_string();

            report(|| {
                println!(
                    "\t🪡 ({}) found {} @ {}",
                    target, entry.symbol, rel_import_path
                );
            });

            if grouped_entries
                .get(&target)
                .and_then(|inner_map| inner_map.get(&entry.key))
                .is_some()
            {
                return Err(anyhow!(
                    "Error: duplicate key found {:?}",
                    (target, entry.key)
                ));
            }

            grouped_entries
                .entry(target.clone())
                .or_default() // Ensures the inner map exists
                .entry(entry.key.clone())
                .insert_entry(WhitelabelRecord {
                    target: target.to_string(),
                    key: entry.key,
                    symbol: WhitelabelSymbol::Symbol {
                        symbol: entry.symbol,
                        import_path: rel_import_path,
                    },
                    remark: entry._experiment_remark,
                });
        }

        let targets = grouped_entries.keys().cloned().collect::<Vec<String>>();

        for entry in wildcards {
            let pb = PathBuf::from(entry.import_path);

            let relative_pb = pb.strip_prefix(root_dir.clone()).unwrap_or(&pb);

            let rel_import_path = relative_pb.to_string_lossy().to_string();

            report(|| {
                println!("\t🪡 (*) found {} @ {}", entry.symbol, rel_import_path);
            });

            for target in &targets {
                if grouped_entries
                    .get(target)
                    .and_then(|inner_map| inner_map.get(&entry.key))
                    .is_none()
                {
                    grouped_entries
                        .entry(target.clone())
                        .or_default()
                        .entry(entry.key.clone())
                        .insert_entry(WhitelabelRecord {
                            target: target.to_owned(),
                            symbol: WhitelabelSymbol::Symbol {
                                symbol: entry.symbol.clone(),
                                import_path: rel_import_path.clone(),
                            },
                            key: entry.key.clone(),
                            remark: entry._experiment_remark.clone(),
                        });
                }
            }
        }

        for entry in optional {
            let WhitelabelTarget::Targetted(target) = entry.target.clone() else {
                return Err(anyhow!(
                    "Optional cannot co-exists with wildcard: {}",
                    entry.target
                ));
            };
            let pb = PathBuf::from(entry.import_path);

            let relative_pb = pb.strip_prefix(root_dir.clone()).unwrap_or(&pb);

            let rel_import_path = relative_pb.to_string_lossy().to_string();

            report(|| {
                println!(
                    "\t🪡 (optional) found {} @ {}",
                    entry.symbol.clone(),
                    rel_import_path
                );
            });

            grouped_entries
                .entry(target.clone())
                .or_default()
                .entry(entry.key.clone())
                .insert_entry(WhitelabelRecord {
                    target: target.to_owned(),
                    symbol: WhitelabelSymbol::Symbol {
                        symbol: entry.symbol.clone(),
                        import_path: rel_import_path.clone(),
                    },
                    key: entry.key.clone(),
                    remark: entry._experiment_remark.clone(),
                });

            for target in &targets {
                if grouped_entries
                    .get(target)
                    .and_then(|inner_map| inner_map.get(&entry.key))
                    .is_none()
                {
                    grouped_entries
                        .entry(target.clone())
                        .or_default()
                        .entry(entry.key.clone())
                        .insert_entry(WhitelabelRecord {
                            target: target.to_owned(),
                            symbol: WhitelabelSymbol::Undefined,
                            key: entry.key.clone(),
                            remark: "undefined".into(),
                        });
                }
            }
        }

        let all_keys: HashSet<&String> = grouped_entries
            .values()
            .flat_map(|inner_map| inner_map.keys())
            .collect();

        let missing_keys: Vec<(&Target, &Key)> = grouped_entries
            .iter()
            .flat_map(|(target, inner_map)| {
                all_keys
                    .iter()
                    .filter(move |&&key| !inner_map.contains_key(key))
                    .map(move |&key| (target, key))
            })
            .collect();

        if !missing_keys.is_empty() {
            return Err(anyhow!(
                "[Registry module] Please check an implementation for following keys:\n{}",
                missing_keys
                    .iter()
                    .map(|(variant, key)| format!("({variant}) {key}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        let mut pivoted: HashMap<Key, HashSet<Target>> = HashMap::new();

        for inner_map in grouped_entries.values() {
            for (key, record) in inner_map.iter() {
                pivoted
                    .entry(key.clone())
                    .or_default()
                    .insert(record.target.clone()); // Insert target string, not the record!
            }
        }

        Ok(WhitelabelRegistry {
            table: grouped_entries,
            pivoted,
        })
    }
}

impl IntoIterator for WhitelabelRegistry {
    type Item = (Target, Vec<WhitelabelRecord>);
    type IntoIter = WhitelabelRegistryIntoIter;

    fn into_iter(self) -> Self::IntoIter {
        WhitelabelRegistryIntoIter {
            inner: self.table.into_iter(),
        }
    }
}

pub struct WhitelabelRegistryIntoIter {
    inner: std::collections::hash_map::IntoIter<Target, HashMap<Key, WhitelabelRecord>>,
}

impl Iterator for WhitelabelRegistryIntoIter {
    type Item = (Target, Vec<WhitelabelRecord>);

    fn next(&mut self) -> Option<Self::Item> {
        // Advance the internal map iterator
        let (target, inner_map) = self.inner.next()?;

        // Transform the inner map into a flat Vec of records
        let records = inner_map.into_values().collect();

        Some((target, records))
    }
}
