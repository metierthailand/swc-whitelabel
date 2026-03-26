use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use swc_core::common::Spanned;
use swc_core::common::{SourceMap, sync::Lrc};
use swc_core::ecma::{
    ast::*,
    visit::{Visit, VisitWith},
};

use crate::ast::collector::WhitelabelEntry;
use crate::config::env;
use crate::util::report;

// Scans the file for imports or local declarations that match known whitelabel symbols
pub struct SymbolScanner<'a> {
    pub source_map: Lrc<SourceMap>,
    pub global_symbols: &'a HashMap<String, Vec<WhitelabelEntry>>,
    pub target_ids: HashMap<Id, String>,
    pub path_mapping: &'a HashMap<String, Vec<String>>,
    current_file_name: Option<Lrc<swc_core::common::FileName>>,
}

impl<'a> SymbolScanner<'a> {
    pub fn new(
        global_symbols: &'a HashMap<String, Vec<WhitelabelEntry>>,
        source_map: Lrc<SourceMap>,
        path_mapping: &'a HashMap<String, Vec<String>>,
    ) -> Self {
        Self {
            global_symbols,
            source_map,
            path_mapping,
            target_ids: HashMap::new(),
            current_file_name: None,
        }
    }

    /// Resolves an import string into an absolute physical file path
    /// Fully respects relative imports and tsconfig.json `paths` aliases.
    fn resolve_import(&self, current_file_path: PathBuf, import_src: &str) -> Option<PathBuf> {
        let cwd = env::with_config(|cfg| cfg.cwd.clone());
        let mut base_paths_to_try = Vec::new();

        // 🎯 CATEGORY 1: Relative Import (Bypasses TS paths)
        if import_src.starts_with('.')
            && let Some(parent) = current_file_path.parent()
        {
            base_paths_to_try.push(parent.join(import_src));
        }
        /* 🎯 CATEGORY 2: TSConfig Path Resolution */
        // Step 1: Check for an EXACT match (e.g., "@/app/whitelabel")
        else if let Some(mapped_paths) = self.path_mapping.get(import_src) {
            for mapped_path in mapped_paths {
                base_paths_to_try.push(cwd.join(mapped_path));
            }
        }
        // Step 2: Check for a WILDCARD match (e.g., "@app/*")
        else if let Some((pattern, mapped_paths, _)) = self.best_path_mapping_match(import_src)
            && let Some(star_idx) = pattern.find('*')
        {
            let prefix = &pattern[..star_idx];
            let suffix = &pattern[star_idx + 1..];

            // Extract the string that replaces the '*'
            let wildcard_match = &import_src[prefix.len()..import_src.len() - suffix.len()];

            for mapped_path in mapped_paths {
                // Inject the matched string into the mapped path's '*'
                let resolved_mapped = mapped_path.replace("*", wildcard_match);
                base_paths_to_try.push(cwd.join(resolved_mapped));
            }
        } else {
            return None;
        }

        // TODO: remove resolution pass
        // 🎯 RESOLUTION PASS: The "Guess the Extension" Game
        let extensions = ["", "ts", "tsx", "js", "jsx", "./index.ts", "./index.tsx"];
        for base_path in base_paths_to_try {
            for ext in extensions {
                let attempt = if ext.contains("/") {
                    base_path.join(ext)
                } else {
                    base_path.with_added_extension(ext)
                };

                if attempt.exists() {
                    // canonicalize() mathematically resolves `../` and `./`
                    return attempt.canonicalize().ok();
                }
            }
        }

        None
    }

    fn best_path_mapping_match(&self, import_src: &str) -> Option<(&String, &Vec<String>, usize)> {
        let mut best_match: Option<(&String, &Vec<String>, usize)> = None;
        for (pattern, mapped_paths) in self.path_mapping {
            if let Some(star_idx) = pattern.find('*') {
                let prefix = &pattern[..star_idx];
                let suffix = &pattern[star_idx + 1..];

                if import_src.starts_with(prefix) && import_src.ends_with(suffix) {
                    let match_len = prefix.len() + suffix.len();
                    // TypeScript Rule: Longest prefix match wins!
                    if best_match.is_none_or(|best| match_len > best.2) {
                        best_match = Some((pattern, mapped_paths, match_len));
                    }
                }
            }
        }

        best_match
    }
}

impl<'a> Visit for SymbolScanner<'a> {
    fn visit_program(&mut self, node: &Program) {
        self.current_file_name = Some(
            self.source_map
                .lookup_char_pos(node.span_lo())
                .file
                .name
                .clone(),
        );
        node.visit_children_with(self);
    }
    fn visit_import_decl(&mut self, import: &ImportDecl) {
        // 1. Grab the raw string (e.g., "./foo" or "@repo/foo")
        let Some(import_src) = import.src.value.as_str() else {
            return;
        };

        // 2. Discriminate based on the Node.js resolution rules
        let mut lazy_resolved_path: Option<PathBuf> = None;

        // 2. Process specifiers and strictly compare paths!
        for specifier in &import.specifiers {
            if let ImportSpecifier::Named(named) = specifier {
                let imported_name = match &named.imported {
                    Some(ModuleExportName::Ident(ident)) => ident.sym.to_string(),
                    Some(ModuleExportName::Str(s)) => s
                        .value
                        .as_str()
                        .unwrap_or_else(|| panic!("Malformed ModuleExportName::Str({:?})", s))
                        .into(),
                    None => named.local.sym.to_string(),
                };

                // 3. MATHEMATICAL CERTAINTY: Does the absolute resolved path exactly match
                // the file where the symbol was originally collected?
                if let Some(entries) = self.global_symbols.get(&imported_name) {
                    let resolved_path = match &lazy_resolved_path {
                        Some(r) => r,
                        None => {
                            lazy_resolved_path =
                                self.current_file_name
                                    .as_ref()
                                    .and_then(|current_file_name| {
                                        self.resolve_import(
                                            current_file_name.to_string().into(),
                                            import_src,
                                        )
                                    });
                            match lazy_resolved_path.as_ref() {
                                Some(resolved_path) => resolved_path,
                                None => {
                                    report(|| {
                                        if let Some(file_name) = self.current_file_name.as_ref() {
                                            println!(
                                                "\t ⚠️ [Warning] Could not resolve import '{}' in {}. Skipping...",
                                                import_src, file_name
                                            );
                                        }
                                    });
                                    continue;
                                }
                            }
                        }
                    };

                    if let Some(entry) =
                        entries
                            .iter()
                            .find(|entry| match fs::canonicalize(resolved_path) {
                                Ok(abs_resolved_path) => {
                                    let absolute_import_path = env::with_config(|cfg| {
                                        cfg.cwd.join(&cfg.src).join(&entry.import_path)
                                    });
                                    let match_exact = match fs::canonicalize(&absolute_import_path)
                                    {
                                        Ok(path) => path == abs_resolved_path,
                                        _ => false,
                                    };

                                    let match_parent = match fs::canonicalize(&absolute_import_path)
                                        .map(|pb| pb.parent().map(|parent| parent.to_path_buf()))
                                    {
                                        Ok(parent) => {
                                            parent.unwrap_or_default() == abs_resolved_path
                                        }
                                        _ => false,
                                    };

                                    match_exact || match_parent
                                }
                                _ => false,
                            })
                    {
                        report(|| {
                            if let Some(file_name) = self.current_file_name.as_ref() {
                                println!("\t 📡 (📦) {} @ {}", entry.key, file_name)
                            }
                        });
                        self.target_ids
                            .insert(named.local.to_id(), entry.key.clone());
                    }
                }
            }
        }
    }

    fn visit_var_declarator(&mut self, decl: &VarDeclarator) {
        if let Pat::Ident(ident) = &decl.name
            && let name = ident.id.sym.to_string()
            && let Some(entries) = self.global_symbols.get(&name)
            && let Some(entry) = entries.iter().find(|e| {
                let absolute_import_path =
                    env::with_config(|cfg| cfg.cwd.join(&cfg.src).join(&e.import_path));

                // FIXME: There can be case where None = None
                self.current_file_name
                    .as_ref()
                    .and_then(|file| fs::canonicalize(file.to_string()).ok())
                    == fs::canonicalize(&absolute_import_path).ok()
            })
        {
            report(|| {
                if let Some(file_name) = self.current_file_name.as_ref() {
                    println!("\t 📡 (🏠) {} @ {}", entry.key, file_name)
                }
            });
            self.target_ids.insert(ident.id.to_id(), entry.key.clone());
        }

        decl.visit_children_with(self);
    }
}
