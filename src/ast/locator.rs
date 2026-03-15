use swc_core::common::Span;
use swc_core::ecma::{
    ast::*,
    visit::{Visit, VisitWith},
};

use swc_ecma_utils::contains_ident_ref;

/// The core struct that traverses the AST to collect the exact Spans of variable usages.
pub struct LocationCollector {
    /// A set of variable names we are tracking (e.g., ["thDescriptionText", "enDescriptionText"])
    pub target_vars: Vec<Ident>,

    /// A list of exact AST Spans where these variables are used
    pub references: Vec<Span>,
}

impl Visit for LocationCollector {
    // 🎯 RULE 1: Catch standard variable usage (e.g., `console.log(thDescriptionText)`)
    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::Ident(ident) = expr {
            if self.target_vars.iter().any(|i| contains_ident_ref(expr, i)) {
                self.references.push(ident.span);
            }
        }
        expr.visit_children_with(self);
    }

    // 🎯 RULE 2: Catch object shorthand usage (e.g., `{ thDescriptionText }`)
    // This is a crucial edge case. It's an object property, but it ALSO reads the local variable!
    fn visit_prop(&mut self, prop: &Prop) {
        if let Prop::Shorthand(ident) = prop {
            if self.target_vars.iter().any(|i| contains_ident_ref(prop, i)) {
                self.references.push(ident.span);
            }
        }
        prop.visit_children_with(self);
    }

    // 🎯 RULE 3: Ignore object properties (e.g., `whitelabel.thDescriptionText`)
    fn visit_member_expr(&mut self, member: &MemberExpr) {
        // Only visit the object part (the left side of the dot).
        member.obj.visit_with(self);
    }

    // 🎯 RULE 4: Ignore object literal keys (e.g., `{ thDescriptionText: "value" }`)
    fn visit_prop_name(&mut self, _name: &PropName) {
        // Do nothing!
    }

    // 🎯 RULE 5: Ignore variable declarations (e.g., `const thDescriptionText = ...`)
    fn visit_pat(&mut self, pat: &Pat) {
        match pat {
            Pat::Ident(_) => {}                 // Ignore the declaration
            _ => pat.visit_children_with(self), // Keep digging for destructuring usages
        }
    }
}

/// The helper function to extract raw AST Spans
pub fn collect_var_reference_spans(program: &Program, target_vars: Vec<Ident>) -> Vec<Span> {
    let mut collector = LocationCollector {
        target_vars,
        references: vec![],
    };

    program.visit_with(&mut collector);
    collector.references
}
