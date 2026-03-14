use std::collections::HashMap;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::{
    ast::*,
    visit::{Visit, VisitMut, VisitMutWith, VisitWith, noop_visit_mut_type},
};

// Scans the file for imports or local declarations that match known whitelabel symbols
pub struct SymbolScanner {
    pub global_symbols: HashMap<String, String>,
    pub target_ids: HashMap<Id, String>,
}

impl Visit for SymbolScanner {
    fn visit_import_specifier(&mut self, import: &ImportSpecifier) {
        if let ImportSpecifier::Named(named) = import {
            let imported_name = match &named.imported {
                Some(ModuleExportName::Ident(ident)) => ident.sym.to_string(),
                Some(ModuleExportName::Str(s)) => s.value.as_str().unwrap_or("Unknown_WL").into(),
                None => named.local.sym.to_string(),
            };
            if let Some(key) = self.global_symbols.get(&imported_name) {
                self.target_ids.insert(named.local.to_id(), key.clone());
            }
        }
    }

    fn visit_var_declarator(&mut self, decl: &VarDeclarator) {
        if let Pat::Ident(ident) = &decl.name {
            let name = ident.id.sym.to_string();
            if let Some(key) = self.global_symbols.get(&name) {
                self.target_ids.insert(ident.id.to_id(), key.clone());
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
                module.body.insert(0, import_decl);
            }
        }
    }
}
