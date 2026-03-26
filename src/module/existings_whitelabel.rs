use swc_core::{
    common::{SourceMap, sync::Lrc},
    ecma::{
        parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer},
        visit::VisitWith,
    },
};

use crate::ast::whitelabel::WhitelabelScanner;
use crate::config::env;
use crate::util::report;

pub fn load(cm: &Lrc<SourceMap>) -> WhitelabelScanner {
    // 1. Determine the path to the existing generated file (e.g., `app/whitelabel/triva.generated.tsx`)
    let existing_default_whitelabel = env::with_config(|cfg| {
        cfg.cwd
            .join(&cfg.src)
            .join(&cfg.output_dir)
            .join(format!("{}.generated.tsx", cfg.default_target))
    });

    let mut existing_whitelabel_scanner = WhitelabelScanner::default();

    // 2. Only run the diffing engine if the old generated file actually exists!
    if existing_default_whitelabel.exists() {
        report(|| {
            println!("🔍 Found previous generated registry. Scanning for existing keys...");
        });

        // Parse the old file into an AST (using your existing SWC parser setup)
        if let Ok(fm) = cm.load_file(&existing_default_whitelabel) {
            let lexer = Lexer::new(
                Syntax::Typescript(TsSyntax {
                    tsx: true,
                    no_early_errors: true,
                    ..Default::default()
                }),
                Default::default(),
                StringInput::from(&*fm),
                None,
            );

            let mut parser = Parser::new_from(lexer);

            if let Ok(old_ast) = parser.parse_program() {
                old_ast.visit_with(&mut existing_whitelabel_scanner);
            }
        }
    }
    existing_whitelabel_scanner
}
