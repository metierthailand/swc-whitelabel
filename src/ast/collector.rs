use std::{
    fmt::{self, Debug},
    path::PathBuf,
};

use serde::Serialize;
use swc_core::{
    common::{
        SourceMap, SourceMapper, Span, Spanned,
        comments::{Comments, SingleThreadedComments},
        sync::Lrc,
    },
    ecma::{
        ast::*,
        visit::{Visit, VisitWith},
    },
};

use crate::ast::parser::ast;
use crate::{ast::parser::directive::DirectiveRuleParser, config::env, util};

#[derive(Clone)]
enum WhitelabelTargets {
    Targetted(Vec<String>),
    Wildcard,
}

impl WhitelabelTargets {
    pub fn is_empty(&self) -> bool {
        match self {
            WhitelabelTargets::Targetted(items) => items.is_empty(),
            WhitelabelTargets::Wildcard => false,
        }
    }

    pub fn push(&mut self, i: String) {
        match self {
            WhitelabelTargets::Targetted(items) => {
                items.push(i);
            }
            WhitelabelTargets::Wildcard => {}
        }
    }
}

impl Default for WhitelabelTargets {
    fn default() -> Self {
        Self::Targetted(vec![])
    }
}

#[derive(Clone, Default)]
struct WhitelabelDirective {
    targets: WhitelabelTargets,
    key: Option<String>,
    optional: bool,
}

#[derive(Debug, Clone, Serialize)]
pub enum WhitelabelTarget {
    Targetted(String),
    Wildcard,
}

impl fmt::Display for WhitelabelTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WhitelabelTarget::Targetted(target) => write!(f, "{}", target),
            WhitelabelTarget::Wildcard => write!(f, "*"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WhitelabelEntry {
    pub target: WhitelabelTarget,
    pub key: String,
    pub symbol: String,
    pub import_path: String,
    pub _experiment_remark: String,
    pub optional: bool,
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
            let directive_str = comment.text.trim();

            if directive_str.starts_with("whitelabel") {
                let directive_ast_result = DirectiveRuleParser::new().parse(directive_str);
                if let Ok(directive_ast) = directive_ast_result {
                    let mut parsed_directive: WhitelabelDirective =
                        directive_ast
                            .iter()
                            .fold(Default::default(), |mut wl, opt| {
                                match opt {
                                    ast::Modifier::Optional(b) => wl.optional = b.to_owned(),
                                    ast::Modifier::ForModifier(ast::ForModifier::For(t)) => {
                                        wl.targets.push(t.to_owned());
                                    }
                                    ast::Modifier::ForModifier(ast::ForModifier::Wildcard) => {
                                        wl.targets = WhitelabelTargets::Wildcard
                                    }
                                    ast::Modifier::Key(k) => wl.key = Some(k.clone()),
                                };
                                wl
                            });

                    if parsed_directive.targets.is_empty() {
                        parsed_directive
                            .targets
                            .push(env::with_config(|cfg| cfg.default_target.clone()));
                    }

                    return Some(parsed_directive);
                } else {
                    self.errors
                        .push(format!("Parsing error: {:?}", directive_ast_result.err()));
                    continue;
                };
            }
        }
        None
    }

    /// Dynamically extracts the physical path, import path, and line number from the AST Span
    fn get_filename(&self, span: swc_core::common::Span) -> String {
        let loc = self.source_map.lookup_char_pos(span.lo);

        loc.file.name.to_string()
    }

    fn register(
        &mut self,
        symbol: String,
        final_key: String,
        targets: WhitelabelTargets,
        optional: bool,
        span: Span,
    ) {
        match targets {
            WhitelabelTargets::Targetted(items) => {
                for target in items {
                    self.entries.push(WhitelabelEntry {
                        optional,
                        target: WhitelabelTarget::Targetted(target),
                        key: final_key.clone(),
                        symbol: symbol.clone(),
                        import_path: self.get_filename(span),
                        _experiment_remark: self
                            .source_map
                            .span_to_snippet(span)
                            .unwrap_or_default(),
                    });
                }
            }
            WhitelabelTargets::Wildcard => self.entries.push(WhitelabelEntry {
                optional,
                target: WhitelabelTarget::Wildcard,
                key: final_key.clone(),
                symbol: symbol.clone(),
                import_path: self.get_filename(span),
                _experiment_remark: self.source_map.span_to_snippet(span).unwrap_or_default(),
            }),
        }
    }
}

impl<'a> Visit for WhitelabelCollector<'a> {
    // Catch standard `export const` and `export function`
    fn visit_export_decl(&mut self, export: &ExportDecl) {
        if let Some(WhitelabelDirective {
            targets,
            key,
            optional,
        }) = self.get_whitelabel_target_and_key(export.span)
        {
            match &export.decl {
                Decl::Var(var_decl) => {
                    if let Some(decl) = var_decl.decls.first()
                        && let Pat::Ident(ident) = &decl.name
                    {
                        let symbol = ident.id.sym.to_string();
                        let final_key = key.clone().unwrap_or_else(|| symbol.clone());

                        self.register(symbol, final_key, targets, optional, decl.init.span());
                    }
                }
                Decl::Fn(fn_decl) => {
                    let symbol = fn_decl.ident.sym.to_string();
                    let final_key = key.clone().unwrap_or_else(|| symbol.clone());

                    self.register(symbol, final_key, targets, optional, fn_decl.span());
                }
                _ => {
                    let loc = self.source_map.lookup_char_pos(export.span.lo);

                    let rel_path = env::with_config(|cfg| {
                        util::compute_relative_import(
                            cfg.cwd.clone().as_path(),
                            PathBuf::from(self.get_filename(export.span)).as_path(),
                        )
                    });

                    self.errors.push(format!(
                        "Unsupported export declaration for whitelabel {} @{}",
                        rel_path.unwrap_or(loc.file.name.to_string()),
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
            let rel_path = env::with_config(|cfg| {
                util::compute_relative_import(
                    cfg.cwd.clone().as_path(),
                    PathBuf::from(self.get_filename(export.span)).as_path(),
                )
            });
            self.errors.push(format!(
                "File {} contains a whitelabel directive on a named export block. \
                This is not supported in v1. Use direct inline exports.",
                rel_path.unwrap_or("N/A".to_string())
            ));
        }
        export.visit_children_with(self);
    }
}
