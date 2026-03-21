use std::path::PathBuf;

use serde::Serialize;
use swc_core::{
    common::{
        SourceMap, SourceMapper, Spanned,
        comments::{Comments, SingleThreadedComments},
        sync::Lrc,
    },
    ecma::{
        ast::*,
        visit::{Visit, VisitWith},
    },
};

use crate::{config::config, util};

#[derive(Clone)]
struct WhitelabelDirective {
    targets: Vec<Option<String>>,
    key: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhitelabelEntry {
    pub target: Option<String>,
    pub key: String,
    pub symbol: String,
    pub import_path: String,
    pub _experiment_remark: String,
}

pub struct WhitelabelCollector<'a> {
    source_map: &'a Lrc<SourceMap>,
    comments: &'a SingleThreadedComments,
    pub entries: Vec<WhitelabelEntry>,
    pub errors: Vec<String>,
}

impl<'a> WhitelabelCollector<'a> {
    pub fn new(source_map: &'a Lrc<SourceMap>, comments: &'a SingleThreadedComments) -> Self {
        Self {
            source_map,
            comments,
            entries: vec![],
            errors: vec![],
        }
    }

    /// Robustly scans all leading comments for the whitelabel directive
    fn get_whitelabel_target_and_key(
        &mut self,
        span: swc_core::common::Span,
    ) -> Option<WhitelabelDirective> {
        let leading_comments = self.comments.get_leading(span.lo)?;
        for comment in leading_comments {
            let text = comment.text.trim();
            if let Some(rest) = text.strip_prefix("whitelabel") {
                let directive = rest.trim_start_matches(':');

                let mut targets = Vec::new();
                let mut key = None;

                for part in directive.split(',') {
                    let part = part.trim();
                    if let Some(t) = part.strip_prefix("for=") {
                        targets.push(Some(t.trim().to_string()));
                    } else if let Some(k) = part.strip_prefix("key=") {
                        key = Some(k.trim().to_string());
                    }
                }

                // Smart Fallback: If no `for=` is provided, use the default!
                if targets.is_empty() {
                    targets.push(None);
                }

                return Some(WhitelabelDirective { targets, key });
            }
        }
        None
    }

    /// Dynamically extracts the physical path, import path, and line number from the AST Span
    fn get_filename(&self, span: swc_core::common::Span) -> String {
        let loc = self.source_map.lookup_char_pos(span.lo);

        loc.file.name.to_string()
    }
}

impl<'a> Visit for WhitelabelCollector<'a> {
    // Catch standard `export const` and `export function`
    fn visit_export_decl(&mut self, export: &ExportDecl) {
        if let Some(WhitelabelDirective { targets, key }) =
            self.get_whitelabel_target_and_key(export.span)
        {
            match &export.decl {
                Decl::Var(var_decl) => {
                    if let Some(decl) = var_decl.decls.first()
                        && let Pat::Ident(ident) = &decl.name {
                            let symbol = ident.id.sym.to_string();
                            let final_key = key.clone().unwrap_or_else(|| symbol.clone());

                            // Loop through all targets and push an entry for each!
                            for target in targets {
                                self.entries.push(WhitelabelEntry {
                                    target,
                                    key: final_key.clone(),
                                    symbol: symbol.clone(),
                                    import_path: self.get_filename(export.span),
                                    _experiment_remark: self
                                        .source_map
                                        .span_to_snippet(decl.init.span())
                                        .unwrap_or_default(),
                                });
                            }
                        }
                }
                Decl::Fn(fn_decl) => {
                    let symbol = fn_decl.ident.sym.to_string();
                    let final_key = key.clone().unwrap_or_else(|| symbol.clone());

                    for target in targets {
                        self.entries.push(WhitelabelEntry {
                            target,
                            key: final_key.clone(),
                            symbol: symbol.clone(),
                            import_path: self.get_filename(export.span),
                            _experiment_remark: self
                                .source_map
                                .span_to_snippet(fn_decl.function.span)
                                .unwrap_or_default(),
                        });
                    }
                }
                _ => {
                    let loc = self.source_map.lookup_char_pos(export.span.lo);

                    let rel_path = config::with_config(|cfg| {
                        util::compute_relative_import(
                            cfg.cwd.clone().as_path(),
                            PathBuf::from(self.get_filename(export.span)).as_path(),
                        )
                    });

                    self.errors.push(format!(
                        "Unsupported export declaration for whitelabel {} @{}",
                        rel_path.unwrap(),
                        loc.line
                    ))
                }
            }
        }
        export.visit_children_with(self);
    }

    // Fail loud on re-exports (e.g., `export { foo as companyName }`)
    fn visit_named_export(&mut self, export: &NamedExport) {
        if self.get_whitelabel_target_and_key(export.span).is_some() {
            let rel_path = config::with_config(|cfg| {
                util::compute_relative_import(
                    cfg.cwd.clone().as_path(),
                    PathBuf::from(self.get_filename(export.span)).as_path(),
                )
            });
            self.errors.push(format!(
                "File {} contains a whitelabel directive on a named export block. \
                This is not supported in v1. Use direct inline exports.",
                rel_path.unwrap()
            ));
        }
        export.visit_children_with(self);
    }
}
