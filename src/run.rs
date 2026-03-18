use anyhow::Result;
use glob::{GlobError, glob};
use std::fs;
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
use crate::config;
use crate::generator;
use crate::module;

use crate::ast::whitelabel::WhitelabelScanner;
use crate::util::{create_reporter, report};

pub fn run(cwd: Option<PathBuf>) -> Result<()> {
    //  A central registry of every file we write to disk
    let mut modified_files: Vec<String> = Vec::new();

    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let globals = Globals::new();

    let Ok(_) = config::config::init(cwd, "whitelabel.config.json") else {
        panic!("Failed to load config");
    };

    let cfg = config::config::get();

    let report_modified_files = create_reporter(|c| c.output_file_name_only);

    // 1. Determine the path to the existing generated file (e.g., `app/whitelabel/triva.generated.tsx`)
    let mut existing_default_whitelabel = cfg.cwd.clone();
    existing_default_whitelabel.push(format!(
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

        let mut root_dir = cfg.cwd.clone();
        root_dir.push(&cfg.src);

        for pattern in &cfg.patterns {
            let Ok(paths) = glob(format!("{}{}", root_dir.display(), pattern).as_str()) else {
                panic!("Failed to load {}", pattern)
            };
            for p in paths {
                files.push(p);
            }
        }

        report(|| {
            println!("🔍 Start collecting whitelabel keys ...");
        });

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
            for err in &collector.errors {
                eprintln!("❌ Error: {}", err);
            }
        }

        // Group entries by target (e.g., trivacafe, martech)
        let mut grouped_entries: HashMap<String, Vec<ast::collector::WhitelabelEntry>> =
            HashMap::new();
        let mut rename_map: HashMap<String, String> = HashMap::new();
        for entry in &collector.entries {
            let pb = PathBuf::from(&entry.import_path);

            let import_path = format!(
                "{}",
                pb.with_extension("")
                    .strip_prefix(&cfg.src)
                    .unwrap_or(&pb)
                    .display()
            );

            let rewritten_entry = ast::collector::WhitelabelEntry {
                target: Some(entry.target.clone().unwrap_or(cfg.default_target.clone())),
                import_path,
                ..(entry.clone())
            };

            if let Some(prev_key) = existing_whitelabel_scanner.symbol_to_key.get(&entry.symbol) {
                if prev_key != &entry.key && entry.target == Some(cfg.default_target.clone()) {
                    report(|| {
                        println!(
                            "\t ⚠️ Detected renamed directive: '{}' -> '{}'",
                            prev_key, entry.key
                        );
                    });
                    rename_map.insert(prev_key.clone(), entry.key.clone());
                }
            }
            report(|| {
                println!(
                    "\t🪡 ({}) found {} @ {}",
                    rewritten_entry.target.clone().unwrap_or_default(),
                    entry.symbol,
                    entry.import_path
                );
            });

            grouped_entries
                .entry(rewritten_entry.target.clone().unwrap())
                .or_default()
                .push(rewritten_entry);
        }

        report(|| {
            println!("🏗️ Starting whitelabel code generation...",);
        });

        let mut output_dir = cfg.cwd.clone();
        output_dir.push(&cfg.src);
        output_dir.push(&cfg.output_dir);
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

        let target_path = format!("{}/index.ts", output_dir.display());
        fs::write(
            &target_path,
            generator::index::generate(
                grouped_entries.iter().map(|(target, _)| target).collect(),
                cfg.default_target.clone(),
            ),
        )?;

        modified_files.push(target_path);

        let determiner = PathBuf::from(format!("{}/determine-whitelabel.ts", output_dir.display()));

        if determiner.exists() {
            report(|| {
                println!(
                    "🙈 Detected {}, skipped code generation.",
                    determiner.display()
                );
            });
        } else {
            fs::write(
                determiner,
                generator::determines_whitelabel::generate(cfg.default_target.clone()),
            )?;
            modified_files.push(format!("{}/determine-whitelabel.ts", output_dir.display()));
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
