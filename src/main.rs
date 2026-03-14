use anyhow::Result;
use glob::glob;
use std::collections::HashMap;
use std::fs;
use swc_core::{
    common::{
        Mark, SourceMap,
        comments::SingleThreadedComments,
        errors::{ColorConfig, Handler},
        sync::Lrc,
    },
    ecma::{
        ast::*,
        codegen::{Emitter, text_writer::JsWriter},
        parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer},
        transforms::base::resolver,
        visit::{VisitMutWith, VisitWith},
    },
};

use swc_core::common::{GLOBALS, Globals};

mod codemod;
mod collector;

fn main() -> Result<()> {
    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let globals = Globals::new();

    GLOBALS.set(&globals, || {
        let mut all_entries: Vec<collector::WhitelabelEntry> = vec![];
        let mut has_errors = false;
        let files: Vec<_> = glob("app/**/*.tsx")
            .unwrap()
            .chain(glob("app/**/*.ts").unwrap())
            .collect();

        // Scan for TSX files
        for entry in &files {
            let path = entry.as_ref().unwrap();

            // Skip the generated file to avoid infinite loops
            if path.ends_with("whitelabel.generated.tsx") {
                continue;
            }

            let fm = cm.load_file(&path)?;
            let comments = SingleThreadedComments::default();

            let lexer = Lexer::new(
                Syntax::Typescript(TsSyntax {
                    tsx: true,
                    ..Default::default()
                }),
                Default::default(),
                StringInput::from(&*fm),
                Some(&comments),
            );

            let mut parser = Parser::new_from(lexer);
            let module = match parser.parse_module() {
                Ok(m) => m,
                Err(e) => {
                    e.into_diagnostic(&handler).emit();
                    continue;
                }
            };

            // Format import path (e.g., "src/components/branding.tsx" -> "./components/branding")
            let import_path = format!(
                "./{}",
                path.with_extension("")
                    .strip_prefix("src/")
                    .unwrap_or(&path)
                    .display()
            );

            let mut collector = collector::WhitelabelCollector::new(&comments, import_path);
            module.visit_with(&mut collector);

            if !collector.errors.is_empty() {
                for err in collector.errors {
                    eprintln!("❌ Error in {}: {}", path.display(), err);
                }
                has_errors = true;
            }

            all_entries.extend(collector.entries);
        }

        if has_errors {
            anyhow::bail!("Whitelabel extraction failed due to authoring errors.");
        }

        // Generate the output file
        all_entries.sort_by(|a, b| a.key.cmp(&b.key));

        let mut output = String::new();
        output.push_str("// AUTO-GENERATED: DO NOT EDIT\n\n");

        // Generate Imports
        for entry in &all_entries {
            output.push_str(&format!(
                "import {{ {} }} from \"{}\";\n",
                entry.symbol, entry.import_path
            ));
        }

        // Generate Object
        output.push_str("\nconst whitelabel = {\n");
        for entry in &all_entries {
            if entry.key == entry.symbol {
                output.push_str(&format!("  {},\n", entry.key));
            } else {
                output.push_str(&format!("  {}: {},\n", entry.key, entry.symbol));
            }
        }
        output.push_str("};\n\nexport default whitelabel;\n");

        fs::write("app/whitelabel.generated.tsx", output)?;
        println!(
            "✅ Successfully generated src/whitelabel.generated.tsx with {} entries.",
            all_entries.len()
        );
        // -----------------------------------------------------------------------------
        // Codemod Pass: Rewrite References Across All Files
        // -----------------------------------------------------------------------------
        println!("🚀 Starting codemod pass to rewrite references...");
        let mut global_symbols = HashMap::new();
        for entry in &all_entries {
            global_symbols.insert(entry.symbol.clone(), entry.key.clone());
        }

        for entry in files {
            let path = entry?;
            if path.ends_with("whitelabel.generated.tsx") {
                continue;
            }

            let fm = cm.load_file(&path)?;
            let comments = SingleThreadedComments::default();
            let lexer = Lexer::new(
                Syntax::Typescript(TsSyntax {
                    tsx: true,
                    ..Default::default()
                }),
                Default::default(),
                StringInput::from(&*fm),
                Some(&comments),
            );
            let mut parser = Parser::new_from(lexer);
            let mut program = match parser.parse_program() {
                Ok(p) => p,
                Err(_) => continue,
            };

            let unresolved_mark = Mark::new();
            let top_level_mark = Mark::new();

            program.visit_mut_with(&mut resolver(unresolved_mark, top_level_mark, false));

            use swc_core::ecma::visit::VisitWith;
            let mut scanner = codemod::SymbolScanner {
                global_symbols: global_symbols.clone(),
                target_ids: HashMap::new(),
            };
            program.visit_with(&mut scanner);

            if scanner.target_ids.is_empty() {
                continue;
            }

            let mut rewriter = codemod::WhitelabelRewriter {
                target_ids: scanner.target_ids,
                has_modified: false,
            };
            program.visit_mut_with(&mut rewriter);

            if rewriter.has_modified {
                let mut buf = vec![];
                let mut emitter = Emitter {
                    cfg: swc_core::ecma::codegen::Config::default()
                        .with_target(EsVersion::Es2022)
                        .with_omit_last_semi(true),
                    cm: cm.clone(),
                    comments: Some(&comments),
                    wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
                };
                emitter.emit_program(&program)?;
                fs::write(&path, String::from_utf8(buf)?)?;
                println!("✍️  Rewrote references in {}", path.display());
            }
        }
        Ok(())
    })
}
