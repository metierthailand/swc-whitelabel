use anyhow::{Result, anyhow};
use glob::{GlobError, glob};
use std::fs;
use std::path::PathBuf;
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

use crate::generator;
use crate::{ast, module::registry::WhitelabelRegistry};
use crate::{config, module};

use crate::util::{create_reporter, report};

pub fn run(cwd: Option<PathBuf>) -> Result<()> {
    config::env::init(cwd, "whitelabel.config.json")?;

    //  A central registry of every file we write to disk
    let mut modified_files: Vec<String> = Vec::new();

    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let globals = Globals::new();

    let cfg = config::env::with_config(|c| c.clone());

    let report_modified_files = create_reporter(|c| c.output_file_name_only);

    // TODO: renaming detection
    // let _existing_whitelabel_scanner = module::existings_whitelabel::load(&cm);
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
                    e.clone().into_diagnostic(&handler).emit();
                    return Err(anyhow!("Error while parsing module: {:?}", e));
                }
            };

            module.visit_with(&mut collector);
        }

        if !collector.errors.is_empty() {
            for err in &collector.errors {
                eprintln!("❌ Error: {}", err);
            }
            return Err(anyhow!("{:?}", collector.errors));
        }

        let len = collector.entries.len();
        let registry: WhitelabelRegistry = collector.try_into()?;

        report(|| {
            println!("🏗️ Starting whitelabel code generation...",);
        });

        let output_dir = root_dir.join(&cfg.output_dir);
        fs::create_dir_all(&output_dir)?;

        let target_path = output_dir.join("whitelabel.ts");
        fs::write(&target_path, generator::whitelabel::generate(&registry))?;
        modified_files.push(target_path.to_string_lossy().to_string());

        for (target, entry) in registry.clone().into_iter() {
            let output = generator::wl::generate(entry);
            let target_path = format!("{}/{}.generated.tsx", output_dir.display(), target);
            fs::write(&target_path, output)?;

            report(|| {
                println!("\t💼 {} ✅", target_path);
            });

            modified_files.push(target_path);
        }

        let wrapper = output_dir.join("index.ts");

        if wrapper.exists() {
            report(|| {
                println!(
                    "🙈 Detected {}, skipped code generation.",
                    wrapper.display()
                );
            });
        } else {
            fs::write(
                &wrapper,
                generator::index::generate(cfg.default_target.clone()),
            )?;
            modified_files.push(wrapper.to_string_lossy().to_string());
        }

        report(|| {
            println!(
                "✅ Successfully generated whitelabel registry in {}/ with {} total entries.",
                output_dir.display(),
                len
            );
            println!("🪄 Starting codemod pass to rewrite references...");
        });

        // -----------------------------------------------------------------------------
        // Codemod Pass: Rewrite References Across All Files
        // -----------------------------------------------------------------------------
        let codemod_modified_files = module::codemod::exec(&cm, &registry)?;

        modified_files.extend(codemod_modified_files);

        // TODO: renaming detection
        // if !rename_map.is_empty() {
        //     let renamed_files = module::rename_whitelabel::exec(&cm, &rename_map);
        //     modified_files.extend(renamed_files);
        // }

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
