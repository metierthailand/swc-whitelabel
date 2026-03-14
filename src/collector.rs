// use swc_core::ecma::visit::VisitMutWith;
use swc_core::{
    common::{
        SourceMap,
        comments::{Comments, SingleThreadedComments},
        sync::Lrc,
    },
    ecma::{
        ast::*,
        visit::{Visit, VisitWith},
    },
};

#[derive(Clone)]
struct WhitelabelDirective {
    target: Option<String>,
    key: Option<String>,
}

#[derive(Debug)]
pub struct WhitelabelEntry {
    pub target: String,
    pub key: String,
    pub symbol: String,
    pub import_path: String,
}

pub struct WhitelabelCollector<'a> {
    source_map: &'a Lrc<SourceMap>,
    comments: &'a SingleThreadedComments,
    file_path: String,
    default_target: String,
    pub entries: Vec<WhitelabelEntry>,
    pub errors: Vec<String>,
}

impl<'a> WhitelabelCollector<'a> {
    pub fn new(
        source_map: &'a Lrc<SourceMap>,
        comments: &'a SingleThreadedComments,
        file_path: String,
        default_target: String,
    ) -> Self {
        Self {
            source_map,
            comments,
            file_path,
            default_target,
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
            if let Some(rest) = text.strip_prefix("whitelabel:") {
                let directive = rest.trim();

                let mut target = None;
                let mut key = None;

                for part in directive.split(',') {
                    let part = part.trim();
                    if let Some(t) = part.strip_prefix("for=") {
                        target = Some(t.trim().to_string());
                    } else if let Some(k) = part.strip_prefix("key=") {
                        key = Some(k.trim().to_string());
                    }
                }

                if target.is_none() {
                    self.errors.push(format!(
                        "Invalid directive '{}'. Must use 'for=target'.",
                        directive
                    ));
                    return None;
                }

                return Some(WhitelabelDirective { target, key });
            }
        }
        None
    }
}

impl<'a> Visit for WhitelabelCollector<'a> {
    // Catch standard `export const` and `export function`
    fn visit_export_decl(&mut self, export: &ExportDecl) {
        if let Some(WhitelabelDirective { target, key }) =
            self.get_whitelabel_target_and_key(export.span)
        {
            match &export.decl {
                Decl::Var(var_decl) => {
                    if let Some(decl) = var_decl.decls.first() {
                        if let Pat::Ident(ident) = &decl.name {
                            self.entries.push(WhitelabelEntry {
                                target: target.unwrap_or(self.default_target.clone()),
                                key: key.unwrap_or(ident.id.sym.to_string()),
                                symbol: ident.id.sym.to_string(),
                                import_path: self.file_path.clone(),
                            });
                        }
                    }
                }
                Decl::Fn(fn_decl) => {
                    self.entries.push(WhitelabelEntry {
                        target: target.unwrap_or(self.default_target.clone()),
                        key: key.unwrap_or(fn_decl.ident.sym.to_string()),
                        symbol: fn_decl.ident.sym.to_string(),
                        import_path: self.file_path.clone(),
                    });
                }
                _ => {
                    let loc = self.source_map.lookup_char_pos(export.span.lo);
                    self.errors.push(format!(
                        "Unsupported export declaration for whitelabel {} @{}",
                        self.file_path, loc.line
                    ))
                }
            }
        }
        export.visit_children_with(self);
    }

    // Fail loud on re-exports (e.g., `export { foo as companyName }`)
    fn visit_named_export(&mut self, export: &NamedExport) {
        if self.get_whitelabel_target_and_key(export.span).is_some() {
            self.errors.push(format!(
                "File {} contains a whitelabel directive on a named export block. \
                This is not supported in v1. Use direct inline exports.",
                self.file_path
            ));
        }
        export.visit_children_with(self);
    }
}
