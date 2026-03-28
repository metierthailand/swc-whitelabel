use anyhow::{Error, Ok, anyhow};
use std::collections::HashMap;
use std::path::PathBuf;
use swc_core::common::Spanned;
use swc_core::common::{SourceMap, sync::Lrc};
use swc_core::ecma::{
    ast::*,
    visit::{Visit, VisitWith},
};

use crate::common::errorable::Errorable;
use crate::common::registry::WhitelabelRegistry;
use crate::util::resolver::TsImportPathResolver;
use crate::util::{cname, report};

// Scans the file for imports or local declarations that match known whitelabel symbols
pub struct SymbolScanner<'a> {
    pub source_map: Lrc<SourceMap>,
    pub registry: &'a WhitelabelRegistry,
    pub target_ids: HashMap<Id, String>,
    pub resolver: &'a TsImportPathResolver,
    current_file_name: Option<Lrc<swc_core::common::FileName>>,
    errors: Vec<Error>,
}

impl<'a> Errorable<HashMap<Id, String>> for SymbolScanner<'a> {
    fn into_result(self) -> anyhow::Result<HashMap<Id, String>> {
        if !self.errors.is_empty() {
            return Err(anyhow!("{}", self.format_multiple_errors(&self.errors)));
        }
        Ok(self.target_ids)
    }
}

impl<'a> SymbolScanner<'a> {
    pub fn new(
        registry: &'a WhitelabelRegistry,
        source_map: Lrc<SourceMap>,
        resolver: &'a TsImportPathResolver,
    ) -> Self {
        Self {
            registry,
            source_map,
            resolver,
            target_ids: HashMap::new(),
            current_file_name: None,
            errors: vec![],
        }
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
            return self.errors.push(anyhow!(
                "[SymbolScanner.visit_import_decl] Couldn't resolve import.src.value: {:?}",
                import.src.value
            ));
        };

        let Some(current_file_name) = &self.current_file_name else {
            return self.errors.push(anyhow!(
                "[SymbolScanner.visit_import_decl] current_file_name is not loaded",
            ));
        };

        // 2. Discriminate based on the Node.js resolution rules
        let Some(resolved_path) = self
            .resolver
            .resolve_import(current_file_name.to_string().into(), import_src)
        else {
            return;
        };

        // 2. Process specifiers and strictly compare paths!
        for specifier in &import.specifiers {
            if let ImportSpecifier::Named(named) = specifier {
                let imported_name = match &named.imported {
                    Some(ModuleExportName::Ident(ident)) => ident.sym.to_string(),
                    Some(ModuleExportName::Str(s)) => match s.value.as_str() {
                        Some(str) => str.into(),
                        None => {
                            return self
                                .errors
                                .push(anyhow!("Malformed ModuleExportName::Str({:?})", s));
                        }
                    },
                    None => named.local.sym.to_string(),
                };

                if let Some(entry) = self
                    .registry
                    .lookup(&imported_name, &resolved_path.with_extension(""))
                {
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
            && let Some(current_file_name) = &self.current_file_name
            && let Some(resolved_current_file_name) =
                cname(PathBuf::from(&current_file_name.to_string()))
            && let Some(entry) = self
                .registry
                .lookup(&name, &resolved_current_file_name.with_extension(""))
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
