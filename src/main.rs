use anyhow::Result;
use glob::{GlobError, glob};
use std::{collections::HashMap, path::PathBuf};
use std::{default, fs};
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

mod ast;
mod config;
mod generator;
mod module;
mod util;

use crate::ast::whitelabel::WhitelabelScanner;
use crate::config::tsconfig;
use crate::util::report;

fn main() -> Result<()> {
    //  A central registry of every file we write to disk
    let mut modified_files: Vec<String> = Vec::new();

    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let globals = Globals::new();

    let Ok(_) = config::config::init() else {
        panic!("Failed to load config");
    };

    let cfg = config::config::get();

    let ts_cfg = match &cfg.tsconfig {
        Some(file) => match tsconfig::load(file.to_string()) {
            Ok(content) => Some(content),
            Err(_) => None,
        },
        None => None,
    };

    // 1. Determine the path to the existing generated file (e.g., `app/whitelabel/triva.generated.tsx`)
    let existing_default_whitelabel = std::path::PathBuf::from(format!(
        "{}{}/{}.generated.tsx",
        cfg.src, cfg.output_dir, cfg.default_target
    ));
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

    GLOBALS.set(&globals, || {
        let mut files: Vec<Result<PathBuf, GlobError>> = vec![];

        for pattern in &cfg.patterns {
            let Ok(paths) = glob(format!("{}{}", cfg.src, pattern).as_str()) else {
                panic!("Failed to load {}", pattern)
            };
            for p in paths {
                files.push(p);
            }
        }

        let comments = SingleThreadedComments::default();

        let mut collector = ast::collector::WhitelabelCollector::new(&cm, &comments);

        // Scan for TSX files
        for entry in &files {
            let path = entry.as_ref().unwrap();

            // Skip the generated file to avoid infinite loops
            if path.to_string_lossy().contains(cfg.output_dir.as_str()) {
                continue;
            }

            let fm = cm.load_file(&path)?;

            let lexer = Lexer::new(
                Syntax::Typescript(TsSyntax {
                    tsx: true,
                    no_early_errors: true,
                    ..Default::default()
                }),
                Default::default(),
                StringInput::from(&*fm),
                Some(&comments),
            );

            // TODO: get_parser
            let mut parser = Parser::new_from(lexer);

            let module = match parser.parse_module() {
                Ok(m) => m,
                Err(e) => {
                    e.into_diagnostic(&handler).emit();
                    continue;
                }
            };

            module.visit_with(&mut collector);
        }

        if !collector.errors.is_empty() {
            for err in collector.errors {
                eprintln!("❌ Error: {}", err);
            }
            anyhow::bail!("Whitelabel extraction failed due to authoring errors.");
        }

        // Group entries by target (e.g., trivacafe, martech)
        let mut grouped_entries: HashMap<String, Vec<ast::collector::WhitelabelEntry>> =
            HashMap::new();
        let mut rename_map: HashMap<String, String> = HashMap::new();
        for entry in &collector.entries {
            report(|| {
                println!(
                    "\t📝 ({}) found {} @{}",
                    // TODO: default to default_target from here
                    entry.target.as_ref().unwrap_or(&cfg.default_target),
                    entry.symbol,
                    entry.import_path
                );
            });

            let pb = PathBuf::from(&entry.import_path);

            let import_path = format!(
                "../{}",
                pb.with_extension("")
                    .strip_prefix(&cfg.src)
                    .unwrap_or(&pb)
                    .display()
            );

            let rewritten_entry = ast::collector::WhitelabelEntry {
                target: entry.target.clone(),
                key: entry.key.clone(),
                symbol: entry.symbol.clone(),
                import_path,
            };

            if let Some(prev_key) = existing_whitelabel_scanner.symbol_to_key.get(&entry.symbol) {
                if prev_key != &entry.key
                    && (entry.target == None || entry.target == Some(cfg.default_target.clone()))
                {
                    report(|| {
                        println!(
                            "\t⚠️ Detected renamed directive: '{}' -> '{}'",
                            prev_key, entry.key
                        );
                    });
                    rename_map.insert(prev_key.clone(), entry.key.clone());
                }
            }

            grouped_entries
                .entry(
                    entry
                        .target
                        .as_ref()
                        .unwrap_or(&cfg.default_target)
                        .to_owned(),
                )
                .or_default()
                .push(rewritten_entry);
        }

        let output_dir = format!("{}{}", cfg.src, cfg.output_dir);
        fs::create_dir_all(&output_dir)?;

        let mut index_exports = String::new();
        let mut index_configs = String::new();

        for (target, entries) in &grouped_entries {
            let output = generator::wl::generate(entries, *target == cfg.default_target);
            let target_path = format!("{}/{}.generated.tsx", output_dir, target);
            fs::write(&target_path, output)?;

            index_exports.push_str(&format!(
                "import {} from \"./{}.generated\";\n",
                target, target
            ));
            index_configs.push_str(&format!("  {},\n", target));
            modified_files.push(target_path);
        }

        let target_path = format!("{}/index.ts", output_dir);
        fs::write(
            &target_path,
            generator::index::generate(index_exports, index_configs, cfg.default_target.clone()),
        )?;

        modified_files.push(target_path);

        report(|| {
            println!(
                "✅ Successfully generated whitelabel registry in {}/ with {} total entries.",
                output_dir,
                collector.entries.len()
            );
            println!("🪄 Starting codemod pass to rewrite references...");
        });

        // -----------------------------------------------------------------------------
        // Codemod Pass: Rewrite References Across All Files
        // -----------------------------------------------------------------------------
        let mut global_symbols = HashMap::new();
        for entry in &collector.entries {
            global_symbols.insert(entry.symbol.clone(), entry.clone());
        }

        for entry in &files {
            let path = entry.as_ref().unwrap();
            if path.to_string_lossy().contains(cfg.output_dir.as_str()) {
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
            let mut scanner = ast::codemod::SymbolScanner::new(
                global_symbols.clone(),
                cm.clone(),
                ts_cfg.clone().unwrap().compiler_options.paths,
            );
            program.visit_with(&mut scanner);

            if scanner.target_ids.is_empty() {
                continue;
            }

            let mut rewriter = ast::codemod::WhitelabelRewriter {
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
                modified_files.push(path.to_string_lossy().to_string());

                report(|| {
                    println!("✅  Rewrote references in {}", path.display());
                });
            }
        }

        if !rename_map.is_empty() {
            // TODO: make others `module` as well.
            let renamed_files = module::rename::rename_whitelabel(&files, &cm, &rename_map);
            modified_files.extend(renamed_files);
        }

        report(|| {
            // Friendly summary for human execution
            println!("🧙🏾‍♂️ Done! Modified {} files.", modified_files.len());
        });

        if cfg.output_file_name_only {
            // Print ONLY the file paths, one per line, so `xargs` can read it perfectly
            for file in modified_files {
                println!("{}", file);
            }
        }

        Ok(())
    })
}
