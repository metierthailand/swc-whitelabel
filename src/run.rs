use anyhow::{Result, anyhow};
use glob::{GlobError, glob};
use std::fs;
use std::ops::Deref;
use std::{collections::HashMap, path::PathBuf};
use swc_core::{
    common::{
        SourceMap,
        comments::SingleThreadedComments,
        errors::{ColorConfig, Handler},
        sync::Lrc,
    },
    ecma::{
        parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer},
        visit::VisitWith,
    },
};

use swc_core::common::{GLOBALS, Globals};

use crate::ast;
use crate::ast::whitelabel::WhitelabelScanner;
use crate::config;
use crate::generator;
use crate::module;

use crate::util::{create_reporter, report};

pub fn run(cwd: Option<PathBuf>) -> Result<()> {
    //  A central registry of every file we write to disk
    let mut modified_files: Vec<String> = Vec::new();

    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let globals = Globals::new();

    let Ok(_) = config::env::init(cwd, "whitelabel.config.json") else {
        panic!("Failed to load config");
    };

    let cfg = config::env::with_config(|c| c.clone());

    let report_modified_files = create_reporter(|c| c.output_file_name_only);

    // 1. Determine the path to the existing generated file (e.g., `app/whitelabel/triva.generated.tsx`)
    let existing_default_whitelabel = cfg
        .cwd
        .join(&cfg.src)
        .join(&cfg.output_dir)
        .join(format!("{}.generated.tsx", cfg.default_target));
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
    let mut files: Vec<Result<PathBuf, GlobError>> = vec![];

    let root_dir = cfg.cwd.join(&cfg.src);

    for pattern in &cfg.patterns {
        let abs = root_dir.join(pattern);
        let Some(Ok(paths)) = abs.to_str().map(glob) else {
            return Err(anyhow!("Failed to load {}", pattern));
        };
        for p in paths {
            files.push(p);
        }
    }

    GLOBALS.set(&globals, move || {
        // let mut root_dir = cfg.cwd.clone();
        // root_dir.push(&cfg.src);

        report(|| {
            println!("🔍 Start collecting whitelabel keys ...");
        });

        let comments = SingleThreadedComments::default();

        let mut collector = ast::collector::WhitelabelCollector::new(&cm, &comments);

        // Scan for TSX files
        for entry in &files {
            let path = match entry.as_ref() {
                Ok(path) => path,
                Err(e) => return Err(anyhow!("Failed to unwrap entry: {:?}", e.error())),
            };

            // Skip the generated file to avoid infinite loops
            if path.to_string_lossy().contains(cfg.output_dir.as_str()) {
                continue;
            }

            let fm = cm.load_file(path)?;

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
            for err in &collector.errors {
                eprintln!("❌ Error: {}", err);
            }
        }

        // Group entries by target (e.g., trivacafe, martech)
        let mut grouped_entries: HashMap<String, Vec<&ast::collector::WhitelabelEntry>> =
            HashMap::new();
        let mut rename_map: HashMap<String, String> = HashMap::new();

        for entry in &mut collector.entries {
            let pb = PathBuf::from(&entry.import_path);

            // Safely strip the absolute project root to guarantee a relative snapshot path
            let relative_pb = pb.strip_prefix(&root_dir).unwrap_or(&pb);

            entry.import_path = relative_pb.to_string_lossy().to_string();
            let actual_target = match entry.target.clone() {
                Some(t) => t,
                None => cfg.default_target.clone(),
            };

            if let Some(prev_key) = existing_whitelabel_scanner.symbol_to_key.get(&entry.symbol)
                && prev_key != &entry.key
                && actual_target == cfg.default_target.as_str()
            {
                report(|| {
                    println!(
                        "\t ⚠️ Detected renamed directive: '{}' -> '{}'",
                        prev_key, entry.key
                    );
                });
                rename_map.insert(prev_key.clone(), entry.key.clone());
            }
            report(|| {
                println!(
                    "\t🪡 ({}) found {} @ {}",
                    actual_target, entry.symbol, entry.import_path
                );
            });

            entry.target = Some(actual_target.deref().to_string());

            grouped_entries
                .entry(actual_target.clone())
                .or_default()
                .push(entry);
        }

        report(|| {
            println!("🏗️ Starting whitelabel code generation...",);
        });

        let output_dir = root_dir.join(&cfg.output_dir);
        fs::create_dir_all(&output_dir)?;

        for (target, entry) in &grouped_entries {
            let output = generator::wl::generate(entry, *target == cfg.default_target);
            let target_path = format!("{}/{}.generated.tsx", output_dir.display(), target);
            fs::write(&target_path, output)?;

            report(|| {
                println!("\t💼 {} ✅", target_path);
            });

            modified_files.push(target_path);
        }

        let target_path = output_dir.join("index.ts");
        fs::write(
            &target_path,
            generator::index::generate(
                grouped_entries.keys().collect(),
                cfg.default_target.clone(),
            ),
        )?;

        modified_files.push(target_path.to_string_lossy().to_string());

        let determiner = output_dir.join("determine-whitelabel.ts");

        if determiner.exists() {
            report(|| {
                println!(
                    "🙈 Detected {}, skipped code generation.",
                    determiner.display()
                );
            });
        } else {
            fs::write(
                &determiner,
                generator::determines_whitelabel::generate(cfg.default_target.clone()),
            )?;
            modified_files.push(determiner.to_string_lossy().to_string());
        }

        report(|| {
            println!(
                "✅ Successfully generated whitelabel registry in {}/ with {} total entries.",
                output_dir.display(),
                collector.entries.len()
            );
            println!("🪄 Starting codemod pass to rewrite references...");
        });

        // -----------------------------------------------------------------------------
        // Codemod Pass: Rewrite References Across All Files
        // -----------------------------------------------------------------------------
        let codemod_modified_files = module::codemod::exec(&cm, &files, collector)?;

        modified_files.extend(codemod_modified_files);

        if !rename_map.is_empty() {
            // TODO: make others `module` as well.
            let renamed_files = module::rename_whitelabel::exec(&files, &cm, &rename_map);
            modified_files.extend(renamed_files);
        }

        report(|| {
            // Friendly summary for human execution
            println!("🧙 Done! Modified {} files.", modified_files.len());
        });

        report_modified_files(Box::new(|| {
            // Print ONLY the file paths, one per line, so `xargs` can read it perfectly
            for file in modified_files {
                println!("{}", file);
            }
        }));

        Ok(())
    })
}
