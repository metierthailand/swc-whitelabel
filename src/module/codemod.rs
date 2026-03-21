use anyhow::Result;
use glob::GlobError;
use std::fs;
use std::{collections::HashMap, path::PathBuf};
use swc_core::{
    common::{Mark, SourceMap, comments::SingleThreadedComments, sync::Lrc},
    ecma::{
        ast::*,
        codegen::{Emitter, text_writer::JsWriter},
        parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer},
        transforms::base::resolver,
        visit::VisitMutWith,
    },
};

use crate::ast::collector::{WhitelabelCollector, WhitelabelEntry};
use crate::ast::rewriter::WhitelabelRewriter;
use crate::ast::scanner::SymbolScanner;
use crate::config::{config, tsconfig};
use crate::util::report;

pub fn exec(
    cm: &Lrc<SourceMap>,
    files: &Vec<std::result::Result<PathBuf, GlobError>>,
    collector: WhitelabelCollector<'_>,
) -> Result<Vec<String>> {
    let mut global_symbols: HashMap<String, Vec<WhitelabelEntry>> = HashMap::new();
    let mut modified_files: Vec<String> = Vec::new();
    let ts_cfg = config::with_config(|cfg| {
        tsconfig::load(cfg.tsconfig.clone().unwrap()).expect("Failed to load tsconfig.json")
    });
    let output_dir = config::with_config(|cfg| cfg.output_dir.clone());

    // 🎯 IDIOMATIC: Consuming the iterator (Value Move)
    for entry in collector.entries.into_iter() {
        global_symbols
            .entry(entry.symbol.clone())
            .or_default()
            .push(entry); // The struct is MOVED, not cloned!
    }

    for entry in files {
        let path = entry.as_ref().unwrap();
        if path.to_string_lossy().contains(output_dir.as_str()) {
            continue;
        }

        let fm = cm.load_file(path)?;
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
        let mut scanner =
            SymbolScanner::new(&global_symbols, cm.clone(), &ts_cfg.compiler_options.paths);
        program.visit_with(&mut scanner);

        if scanner.target_ids.is_empty() {
            continue;
        }

        let mut rewriter = WhitelabelRewriter {
            source_map: cm.clone(),
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
            fs::write(path, String::from_utf8(buf)?)?;
            modified_files.push(path.to_string_lossy().to_string());

            report(|| {
                println!("✅  Rewrote references in {}", path.display());
            });
        }
    }

    Ok(modified_files)
}
