use std::collections::HashMap;
use swc_core::ecma::{
    ast::*,
    visit::{Visit, VisitWith},
};

#[derive(Default)]
pub struct WhitelabelScanner {
    /// Maps the original exported symbol to its old whitelabel key
    /// e.g., "thDescriptionText" -> "blog_thDescriptionText"
    pub symbol_to_key: HashMap<String, String>,
}


impl Visit for WhitelabelScanner {
    fn visit_var_declarator(&mut self, decl: &VarDeclarator) {
        // 1. Check if the variable being declared is exactly "whitelabel"
        if let Pat::Ident(ident) = &decl.name
            && ident.id.sym == *"whitelabel" {
                // 2. Ensure it is initialized with an Object Literal: `{ ... }`
                if let Some(init) = &decl.init
                    && let Expr::Object(obj) = &**init {
                        // 3. Loop through all properties inside the object
                        for prop_or_spread in &obj.props {
                            if let PropOrSpread::Prop(prop) = prop_or_spread {
                                match &**prop {
                                    // 🎯 Case A: Key-Value pairs
                                    // e.g., `blog_thDescriptionText: thDescriptionText`
                                    Prop::KeyValue(kv) => {
                                        // Safely extract the Key (left side)
                                        let key_str = match &kv.key {
                                            PropName::Ident(id) => Some(id.sym.to_string()),
                                            PropName::Str(s) => {
                                                Some(s.value.as_str().unwrap().into())
                                            }
                                            _ => None,
                                        };

                                        // Safely extract the Symbol/Value (right side)
                                        let symbol_str = match &*kv.value {
                                            Expr::Ident(id) => Some(id.sym.to_string()),
                                            _ => None,
                                        };

                                        // If both are valid, map Symbol -> Key
                                        if let (Some(k), Some(s)) = (key_str, symbol_str) {
                                            self.symbol_to_key.insert(s, k);
                                        }
                                    }

                                    // 🎯 Case B: Shorthand properties
                                    // e.g., `EssenceNatureDesktop,`
                                    Prop::Shorthand(ident) => {
                                        let name = ident.sym.to_string();
                                        // For shorthand, symbol and key are identical
                                        self.symbol_to_key.insert(name.clone(), name);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
            }
        // Continue visiting other declarations just in case
        decl.visit_children_with(self);
    }
}
