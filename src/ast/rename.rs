use std::collections::HashMap;
use swc_core::ecma::{
    ast::*,
    visit::{VisitMut, VisitMutWith, noop_visit_mut_type},
};

use crate::util::report;

pub struct WhitelabelRename<'a> {
    pub rename_map: &'a HashMap<String, String>,
    pub has_modified: bool,
}

impl<'a> VisitMut for WhitelabelRename<'a> {
    noop_visit_mut_type!();

    // 🎯 Catch `whitelabel.old_key` and rewrite it to `whitelabel.new_key`
    fn visit_mut_member_expr(&mut self, member: &mut MemberExpr) {
        // Always visit children first in case there are nested expressions
        member.visit_mut_children_with(self);

        // 1. Check if the object being accessed is exactly "whitelabel"
        if let Expr::Ident(obj_ident) = &*member.obj {
            if obj_ident.sym == *"whitelabel" {
                // 2. Check the property being accessed (e.g., `.blog_thDescriptionText`)
                if let MemberProp::Ident(prop_ident) = &mut member.prop {
                    let current_key = prop_ident.sym.to_string();

                    // 3. If this key exists in our rename map, we have a match!
                    if let Some(new_key) = self.rename_map.get(&current_key) {
                        // Surgically overwrite the AST node with the new key
                        prop_ident.sym = new_key.clone().into();

                        self.has_modified = true;

                        report(|| {
                            println!(
                                "✍️  Renamed whitelabel property: {} -> {}",
                                current_key, new_key
                            );
                        })
                    }
                }
            }
        }
    }
}
