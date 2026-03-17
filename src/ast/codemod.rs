use std::collections::HashMap;
use std::path::PathBuf;
use std::{env, fs};
use swc_core::common::Spanned;
use swc_core::common::{DUMMY_SP, SourceMap, sync::Lrc};
use swc_core::ecma::{
    ast::*,
    visit::{Visit, VisitMut, VisitMutWith, VisitWith, noop_visit_mut_type},
};

use crate::ast::collector::WhitelabelEntry;
use crate::util::report;

// Scans the file for imports or local declarations that match known whitelabel symbols
pub struct SymbolScanner {
    pub source_map: Lrc<SourceMap>,
    pub global_symbols: HashMap<String, WhitelabelEntry>,
    pub target_ids: HashMap<Id, String>,
    pub path_mapping: HashMap<String, Vec<String>>,
    current_file_name: Option<Lrc<swc_core::common::FileName>>,
}

impl SymbolScanner {
    pub fn new(
        global_symbols: HashMap<String, WhitelabelEntry>,
        source_map: Lrc<SourceMap>,
        path_mapping: HashMap<String, Vec<String>>,
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
        let mut base_paths_to_try = Vec::new();
        let cwd = env::current_dir().expect("Failed to get current directory");

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
        else if let Some((pattern, mapped_paths, _)) = self.best_path_mapping_match(import_src) {
            let star_idx = pattern.find('*').unwrap();
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
        for (pattern, mapped_paths) in &self.path_mapping {
            if let Some(star_idx) = pattern.find('*') {
                let prefix = &pattern[..star_idx];
                let suffix = &pattern[star_idx + 1..];

                if import_src.starts_with(prefix) && import_src.ends_with(suffix) {
                    let match_len = prefix.len() + suffix.len();
                    // TypeScript Rule: Longest prefix match wins!
                    if best_match.map_or(true, |best| match_len > best.2) {
                        best_match = Some((pattern, mapped_paths, match_len));
                    }
                }
            }
        }

        best_match
    }
}

impl Visit for SymbolScanner {
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
        let import_src = import.src.value.as_str().unwrap();

        // 2. Discriminate based on the Node.js resolution rules
        let Some(resolved_path) = self.resolve_import(
            self.current_file_name.as_ref().unwrap().to_string().into(),
            import_src,
        ) else {
            return;
        };

        // 2. Process specifiers and strictly compare paths!
        for specifier in &import.specifiers {
            if let ImportSpecifier::Named(named) = specifier {
                let imported_name = match &named.imported {
                    Some(ModuleExportName::Ident(ident)) => ident.sym.to_string(),
                    Some(ModuleExportName::Str(s)) => s.value.as_str().unwrap().into(),
                    None => named.local.sym.to_string(),
                };

                // 3. MATHEMATICAL CERTAINTY: Does the absolute resolved path exactly match
                // the file where the symbol was originally collected?
                if let Some(entry) = self.global_symbols.get(&imported_name)
                    && fs::canonicalize(&resolved_path).unwrap()
                        == fs::canonicalize(&entry.import_path).unwrap()
                {
                    report(|| {
                        println!(
                            "🎯 (⬇️) {}@{}",
                            entry.key,
                            self.current_file_name.as_ref().unwrap().to_string()
                        )
                    });
                    self.target_ids
                        .insert(named.local.to_id(), entry.key.clone());
                }
            }
        }
    }

    fn visit_var_declarator(&mut self, decl: &VarDeclarator) {
        if let Pat::Ident(ident) = &decl.name {
            let name = ident.id.sym.to_string();
            if let Some(entry) = self.global_symbols.get(&name)
                && fs::canonicalize(self.current_file_name.as_ref().unwrap().to_string()).unwrap()
                    == fs::canonicalize(&entry.import_path).unwrap()
            {
                report(|| {
                    println!(
                        "🎯 (🔁) {}@{}",
                        entry.key,
                        self.current_file_name.as_ref().unwrap().to_string()
                    )
                });
                self.target_ids.insert(ident.id.to_id(), entry.key.clone());
            }
        }
        decl.visit_children_with(self);
    }
}

pub struct WhitelabelRewriter {
    pub target_ids: HashMap<Id, String>,
    pub has_modified: bool,
}

impl VisitMut for WhitelabelRewriter {
    noop_visit_mut_type!();

    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        expr.visit_mut_children_with(self);
        if let Expr::Ident(ident) = expr {
            if let Some(wl_key) = self.target_ids.get(&ident.to_id()) {
                *expr = Expr::Member(MemberExpr {
                    span: ident.span,
                    obj: Box::new(Expr::Ident(Ident::new(
                        "whitelabel".into(),
                        DUMMY_SP,
                        Default::default(),
                    ))),
                    prop: MemberProp::Ident(IdentName::new(wl_key.clone().into(), DUMMY_SP)),
                });
                self.has_modified = true;
            }
        }
    }

    // Handles shorthand properties (e.g., `{ seedData }` -> `{ seedData: whitelabel.seedData }`)
    fn visit_mut_prop(&mut self, prop: &mut Prop) {
        prop.visit_mut_children_with(self);
        if let Prop::Shorthand(ident) = prop {
            if let Some(wl_key) = self.target_ids.get(&ident.to_id()) {
                *prop = Prop::KeyValue(KeyValueProp {
                    key: PropName::Ident(IdentName::new(ident.sym.clone(), DUMMY_SP)),
                    value: Box::new(Expr::Member(MemberExpr {
                        span: ident.span,
                        obj: Box::new(Expr::Ident(Ident::new(
                            "whitelabel".into(),
                            DUMMY_SP,
                            Default::default(),
                        ))),
                        prop: MemberProp::Ident(IdentName::new(wl_key.clone().into(), DUMMY_SP)),
                    })),
                });
                self.has_modified = true;
            }
        }
    }

    fn visit_mut_program(&mut self, program: &mut Program) {
        program.visit_mut_children_with(self);
        if self.has_modified {
            if let Program::Module(module) = program {
                let import_decl = ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                    span: DUMMY_SP,
                    specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                        span: DUMMY_SP,
                        local: Ident::new("whitelabel".into(), DUMMY_SP, Default::default()),
                    })],
                    src: Box::new(Str {
                        span: DUMMY_SP,
                        value: "@/app/whitelabel".into(),
                        raw: None,
                    }),
                    type_only: false,
                    with: None,
                    phase: Default::default(),
                }));

                // Safely skip Next.js directives like 'use client' or 'use strict'
                let mut insert_idx = 0;
                for item in &module.body {
                    if let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = item {
                        if let Expr::Lit(Lit::Str(s)) = &*expr_stmt.expr {
                            if s.value.starts_with("use ") {
                                insert_idx += 1;
                                continue;
                            }
                        }
                    }
                    break;
                }

                module.body.insert(insert_idx, import_decl);
            }
        }
    }
}
