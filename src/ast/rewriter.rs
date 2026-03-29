use anyhow::{Error, Result, anyhow};
use std::collections::HashMap;
use std::path::PathBuf;
use swc_core::common::Spanned;
use swc_core::common::{DUMMY_SP, SourceMap, sync::Lrc};
use swc_core::ecma::{
    ast::*,
    visit::{VisitMut, VisitMutWith, noop_visit_mut_type},
};

use crate::common::errorable::Errorable;
use crate::config::env;
use crate::util::resolver::TsImportPathResolver;
use crate::util::{self, cname};

pub struct WhitelabelRewriter {
    pub source_map: Lrc<SourceMap>,
    pub target_ids: HashMap<Id, String>,
    pub has_modified: bool,
    resolver: TsImportPathResolver,
    jsx_stack: Vec<JSXElementName>,
    errors: Vec<Error>,
}

const KEYWORD: &str = "whitelabel";

impl Errorable<bool> for WhitelabelRewriter {
    fn into_result(self) -> anyhow::Result<bool> {
        if !self.errors.is_empty() {
            return Err(anyhow!("{}", self.format_multiple_errors(&self.errors)));
        }
        Ok(self.has_modified)
    }
}

impl WhitelabelRewriter {
    pub fn new(
        source_map: Lrc<SourceMap>,
        target_ids: HashMap<Id, String>,
        has_modified: bool,
        resolver: &TsImportPathResolver,
    ) -> Self {
        Self {
            source_map,
            target_ids,
            has_modified,
            resolver: resolver.clone(),
            jsx_stack: vec![],
            errors: vec![],
        }
    }

    fn is_already_imported(&self, i: &ImportDecl) -> Result<bool> {
        let name_matched = i.specifiers.iter().any(|s| match s {
            ImportSpecifier::Named(import_named_specifier) => {
                import_named_specifier.local.sym.eq(KEYWORD)
            }
            ImportSpecifier::Default(import_default_specifier) => {
                import_default_specifier.local.sym.eq(KEYWORD)
            }
            _ => false,
        });
        let current_file_name = self
            .source_map
            .lookup_char_pos(i.span_lo())
            .file
            .name
            .clone();

        if name_matched
            && let Some(import_src) = i.src.value.as_str()
            && let Some(abs_resolved_path) = self
                .resolver
                .resolve_import(current_file_name.to_string().into(), import_src)
            && let Some(whitelabel_import_path) =
                env::with_config(|cfg| cname(cfg.cwd.join(&cfg.src).join(&cfg.output_dir)))
        {
            if abs_resolved_path == whitelabel_import_path {
                return Ok(true);
            }

            let rel_path = env::with_config(|cfg| {
                util::compute_relative_import(
                    cfg.cwd.clone().as_path(),
                    PathBuf::from(&current_file_name.to_string()).as_path(),
                )
                .unwrap_or(current_file_name.to_string())
            });

            return Err(anyhow!(
                "[Rewriter] refused to proceed, found a name {} but difference import @{}",
                KEYWORD,
                rel_path
            ));
        }

        Ok(false)
    }

    fn find_insert_idx(&mut self, module: &Module) -> Option<usize> {
        module.body.iter().try_fold(0, |prev, item| {
            if let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = item
                && let Expr::Lit(Lit::Str(s)) = &*expr_stmt.expr
                && s.value.starts_with("use ")
            {
                return Some(prev + 1);
            } else if let ModuleItem::ModuleDecl(import) = item
                && let ModuleDecl::Import(i) = import
            {
                return match self.is_already_imported(i) {
                    Ok(true) => None,
                    Ok(false) => Some(prev),
                    Err(e) => {
                        self.errors.push(anyhow!(e));
                        None
                    }
                };
            }

            Some(prev)
        })
    }
}

impl VisitMut for WhitelabelRewriter {
    noop_visit_mut_type!();

    fn visit_mut_expr(&mut self, expr: &mut Expr) {
        expr.visit_mut_children_with(self);
        if let Expr::Ident(ident) = expr
            && let Some(wl_key) = self.target_ids.get(&ident.to_id())
        {
            *expr = Expr::Member(MemberExpr {
                span: ident.span,
                obj: Box::new(Expr::Ident(Ident::new(
                    "whitelabel".into(),
                    DUMMY_SP,
                    Default::default(),
                ))),
                prop: MemberProp::Ident(IdentName::new(wl_key.as_str().into(), DUMMY_SP)),
            });
            self.has_modified = true;
        }
    }

    // Handles shorthand properties (e.g., `{ seedData }` -> `{ seedData: whitelabel.seedData }`)
    fn visit_mut_prop(&mut self, prop: &mut Prop) {
        prop.visit_mut_children_with(self);
        if let Prop::Shorthand(ident) = prop
            && let Some(wl_key) = self.target_ids.get(&ident.to_id())
        {
            *prop = Prop::KeyValue(KeyValueProp {
                key: PropName::Ident(IdentName::new(ident.sym.clone(), DUMMY_SP)),
                value: Box::new(Expr::Member(MemberExpr {
                    span: ident.span,
                    obj: Box::new(Expr::Ident(Ident::new(
                        "whitelabel".into(),
                        DUMMY_SP,
                        Default::default(),
                    ))),
                    prop: MemberProp::Ident(IdentName::new(wl_key.as_str().into(), DUMMY_SP)),
                })),
            });
            self.has_modified = true;
        }
    }

    fn visit_mut_jsx_opening_element(&mut self, node: &mut JSXOpeningElement) {
        node.visit_mut_children_with(self);
        if let JSXElementName::Ident(ident) = &node.name
            && let Some(wl_key) = self.target_ids.get(&ident.to_id())
        {
            node.name = JSXElementName::JSXMemberExpr(JSXMemberExpr {
                span: ident.span,
                obj: JSXObject::Ident(Ident::new(
                    "whitelabel".into(),
                    DUMMY_SP,
                    Default::default(),
                )),
                prop: IdentName::new(wl_key.as_str().into(), DUMMY_SP),
            });

            self.has_modified = true;
            if !node.self_closing {
                self.jsx_stack.push(node.name.clone())
            }
        }
    }

    fn visit_mut_jsx_closing_element(&mut self, node: &mut JSXClosingElement) {
        node.visit_mut_children_with(self);
        if let JSXElementName::Ident(ident) = &node.name
            && let Some(_) = self.jsx_stack.last()
            && self.target_ids.contains_key(&ident.to_id())
        {
            node.name = self.jsx_stack.pop().unwrap()
        }
    }

    fn visit_mut_program(&mut self, program: &mut Program) {
        program.visit_mut_children_with(self);
        if self.has_modified
            && let Program::Module(module) = program
        {
            let current_filename: PathBuf = self
                .source_map
                .lookup_char_pos(module.span.lo)
                .file
                .name
                .to_string()
                .into();

            let abs_out_dir =
                env::with_config(|config| config.cwd.join(&config.src).join(&config.output_dir));

            let Some(rel_import) = current_filename
                .as_path()
                .parent()
                .and_then(|path| util::compute_relative_import(path, abs_out_dir.as_path()))
            else {
                return self.errors.push(anyhow!(
                    "[Rewriter] Error while compute relative import between {}, {}",
                    current_filename.display(),
                    abs_out_dir.display()
                ));
            };

            let import_decl = ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                span: DUMMY_SP,
                specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                    span: DUMMY_SP,
                    local: Ident::new("whitelabel".into(), DUMMY_SP, Default::default()),
                })],
                src: Box::new(Str {
                    span: DUMMY_SP,
                    value: rel_import.into(),
                    raw: None,
                }),
                type_only: false,
                with: None,
                phase: Default::default(),
            }));

            // Safely skip Next.js directives like 'use client' or 'use strict'
            let Some(insert_idx) = self.find_insert_idx(module) else {
                return;
            };

            module.body.insert(insert_idx, import_decl);
        }
    }
}
