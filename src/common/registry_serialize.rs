use serde::{Serialize, Serializer};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::config::env::{self, with_config};
use crate::util::compute_relative_import;

use crate::common::registry::{WhitelabelRegistry, WhitelabelSymbol};

impl Serialize for WhitelabelSymbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 1. Define a lightweight proxy that holds references to our strings
        #[derive(Serialize)]
        enum SerializableSymbol {
            // Keep Symbol and Undefined identical
            Symbol {
                symbol: String,
                import_path: String,
                line: usize,
            },
            Undefined,
            // 🚨 FLATTEN THE SYMLINK 🚨
            // Instead of holding the whole Record, just hold the target string!
            Symlink(String),
        }

        let src = with_config(|cfg| cfg.src.clone());

        // 2. Map our real enum to the proxy enum without cloning any data
        let proxy = match self {
            WhitelabelSymbol::Symbol {
                symbol,
                import_path,
                line,
            } => SerializableSymbol::Symbol {
                symbol: symbol.clone(),
                import_path: format!("{}{}", src, import_path.clone()),
                line: *line,
            },
            WhitelabelSymbol::Symlink(record) => SerializableSymbol::Symlink(record.target.clone()),
            WhitelabelSymbol::Undefined => SerializableSymbol::Undefined,
        };

        // 3. Let Serde do the heavy lifting on the proxy
        proxy.serialize(serializer)
    }
}

// Note: Ensure `WhitelabelRecord` and `Loc` have `#[derive(Serialize)]`
impl Serialize for WhitelabelRegistry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct SerializableWhitelabelRecord {
            pub symbol: WhitelabelSymbol,
            pub inline_impl: String,
        }

        #[derive(Serialize)]
        struct LocProxy {
            file: String,
            line: usize,
            column: usize,
        }

        // 1. Define a zero-allocation proxy struct for the JSON output
        #[derive(Serialize)]
        struct ManifestEntry {
            /// Maps a Target (e.g., "def", "v1") to its implementation record
            variants: BTreeMap<String, SerializableWhitelabelRecord>,
            /// The locations where this key is used in the AST
            usages: Vec<LocProxy>,
        }

        // 2. Build the cleanly formatted manifest
        // We group everything strictly by `Key` so the JSON is easy to read.
        let mut manifest: BTreeMap<String, ManifestEntry> = BTreeMap::new();

        let g = self.pivoted.clone();
        for (key, targets) in &g {
            let mut variants = BTreeMap::new();

            let mut sorted_targets: Vec<&String> = targets.iter().collect();

            sorted_targets.sort();

            // Gather all implementations for this key across all targets
            for target in sorted_targets {
                if let Some(record) = self.table.get(target).and_then(|keys| keys.get(key)) {
                    variants.insert(
                        target.clone(),
                        SerializableWhitelabelRecord {
                            symbol: record.symbol.to_owned(),
                            inline_impl: record.remark.to_owned(),
                        },
                    );
                }
            }

            let usages = self
                .usages
                .get(key)
                .map(|locs| {
                    locs.iter()
                        .filter_map(|loc| {
                            if let Some(rel_path) = env::with_config(|cfg| {
                                compute_relative_import(
                                    cfg.cwd.as_path(),
                                    PathBuf::from(loc.file.name.to_string()).as_path(),
                                )
                            }) {
                                return Some(LocProxy {
                                    file: rel_path,
                                    line: loc.line,
                                    column: loc.col_display,
                                });
                            }

                            None
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            manifest.insert(key.clone(), ManifestEntry { variants, usages });
        }

        // 3. Delegate the actual serialization to the built HashMap
        manifest.serialize(serializer)
    }
}
