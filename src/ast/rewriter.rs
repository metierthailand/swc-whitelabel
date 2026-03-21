use std::collections::HashMap;
use std::path::PathBuf;
use swc_core::common::{DUMMY_SP, SourceMap, sync::Lrc};
use swc_core::ecma::{
    ast::*,
    visit::{VisitMut, VisitMutWith, noop_visit_mut_type},
};

use crate::config::config;
use crate::util::{self};

pub struct WhitelabelRewriter {
    pub source_map: Lrc<SourceMap>,
    pub target_ids: HashMap<Id, String>,
    pub has_modified: bool,
    jsx_stack: Vec<JSXElementName>,
}

const KEYWORD: &[u8] = b"whitelabel";

impl WhitelabelRewriter {
    pub fn new(
        source_map: Lrc<SourceMap>,
        target_ids: HashMap<Id, String>,
        has_modified: bool,
    ) -> Self {
        Self {
            source_map,
            target_ids,
            has_modified,
            jsx_stack: vec![],
        }
    }
    fn find_insert_idx(&self, module: &Module) -> Option<usize> {
        module.body.iter().try_fold(0, |prev, item| {
            if let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = item
                && let Expr::Lit(Lit::Str(s)) = &*expr_stmt.expr
                && s.value.starts_with("use ")
            {
                Some(prev + 1)
            } else if let ModuleItem::ModuleDecl(import) = item
                && let ModuleDecl::Import(i) = import
                && i.src
                    .value
                    .as_bytes()
                    .windows(KEYWORD.len())
                    .any(|window| window == KEYWORD)
            {
                None
            } else {
                Some(prev)
            }
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
                prop: MemberProp::Ident(IdentName::new(wl_key.clone().into(), DUMMY_SP)),
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
                    prop: MemberProp::Ident(IdentName::new(wl_key.clone().into(), DUMMY_SP)),
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
                prop: IdentName::new(wl_key.clone().into(), DUMMY_SP),
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
                config::with_config(|config| config.cwd.join(&config.src).join(&config.output_dir));

            let Some(rel_import) = current_filename
                .as_path()
                .parent()
                .and_then(|path| util::compute_relative_import(path, abs_out_dir.as_path()))
            else {
                eprintln!(
                    "[Rewriter] Error while compute relative import between {}, {}",
                    current_filename.display(),
                    abs_out_dir.display()
                );
                return;
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
