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
}

const KEYWORD: &[u8] = b"whitelabel";

impl WhitelabelRewriter {
    fn find_insert_idx(&self, module: &Module) -> Option<usize> {
        module.body.iter().fold(Some(0), |prev, item| {
            if let Some(prev_idx) = prev
                && let ModuleItem::Stmt(Stmt::Expr(expr_stmt)) = item
                && let Expr::Lit(Lit::Str(s)) = &*expr_stmt.expr
                && s.value.starts_with("use ")
            {
                Some(prev_idx + 1)
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
                prev
            }
        })
    }
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

    fn visit_mut_jsx_opening_element(&mut self, node: &mut JSXOpeningElement) {
        node.visit_mut_children_with(self);
        if let JSXElementName::Ident(ident) = &node.name {
            if let Some(wl_key) = self.target_ids.get(&ident.to_id()) {
                *node = JSXOpeningElement {
                    name: JSXElementName::JSXMemberExpr(JSXMemberExpr {
                        span: ident.span,
                        obj: JSXObject::Ident(Ident::new(
                            "whitelabel".into(),
                            DUMMY_SP,
                            Default::default(),
                        )),
                        prop: IdentName::new(wl_key.clone().into(), DUMMY_SP),
                    }),

                    ..node.clone()
                };
                self.has_modified = true;
            }
        }
    }

    fn visit_mut_program(&mut self, program: &mut Program) {
        program.visit_mut_children_with(self);
        if self.has_modified {
            if let Program::Module(module) = program {
                let config = config::get();
                let current_filename: PathBuf = self
                    .source_map
                    .lookup_char_pos(module.span.lo)
                    .file
                    .name
                    .to_string()
                    .into();

                let mut abs_out_dir = config.cwd.clone();
                abs_out_dir.push(format!("{}{}", &config.src, &config.output_dir));

                let import_decl = ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
                    span: DUMMY_SP,
                    specifiers: vec![ImportSpecifier::Default(ImportDefaultSpecifier {
                        span: DUMMY_SP,
                        local: Ident::new("whitelabel".into(), DUMMY_SP, Default::default()),
                    })],
                    src: Box::new(Str {
                        span: DUMMY_SP,
                        value: util::compute_relative_import(
                            current_filename.as_path().parent().unwrap(),
                            abs_out_dir.as_path(),
                        )
                        .unwrap()
                        .into(),
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
}
