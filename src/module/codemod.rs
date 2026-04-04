use anyhow::{Result, anyhow};
use swc_core::common::SourceFile;
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

use crate::ast::rewriter::WhitelabelRewriter;
use crate::ast::scanner::SymbolScanner;
use crate::common::{errorable::Errorable, registry::WhitelabelRegistry};
use crate::config::{env, tsconfig};
use crate::util::report;
use crate::util::resolver::TsImportPathResolver;
use crate::util::transactional::TxFS;

pub fn exec(cm: &Lrc<SourceMap>, registry: &mut WhitelabelRegistry) -> Result<Vec<String>> {
    let mut modified_files: Vec<String> = Vec::new();
    let import_path_resolver = env::with_config(|cfg| {
        let tsconfig = tsconfig::load(cfg.tsconfig.clone())?;
        let resolver: TsImportPathResolver = tsconfig.compiler_options.paths.try_into()?;
        anyhow::Ok(resolver)
    })?;
    let output_dir = env::with_config(|cfg| cfg.output_dir.clone());

    let files: Vec<Lrc<SourceFile>> = {
        // 🔒 Acquire the lock
        let guard = cm.files();

        // Clone the Arcs into a new Vec
        guard.iter().cloned().collect()
    };

    for fm in files {
        if fm.name.to_string().contains(output_dir.as_str()) {
            continue;
        }

        let comments = SingleThreadedComments::default();
        let lexer = Lexer::new(
            Syntax::Typescript(TsSyntax {
                tsx: true,
                ..Default::default()
            }),
            Default::default(),
            StringInput::from(fm.as_ref()),
            Some(&comments),
        );
        let mut parser = Parser::new_from(lexer);
        let mut program = match parser.parse_program() {
            Ok(p) => p,
            Err(e) => return Err(anyhow!("{:?}", e)),
        };

        let unresolved_mark = Mark::new();
        let top_level_mark = Mark::new();

        program.visit_mut_with(&mut resolver(unresolved_mark, top_level_mark, false));

        use swc_core::ecma::visit::VisitWith;
        let mut scanner = SymbolScanner::new(registry, cm.clone(), &import_path_resolver);
        program.visit_with(&mut scanner);

        let target_ids = scanner.into_result()?;

        if target_ids.is_empty() {
            continue;
        }

        let mut rewriter = WhitelabelRewriter::new(
            cm.clone(),
            target_ids,
            false,
            &import_path_resolver,
            registry,
        );
        program.visit_mut_with(&mut rewriter);

        if rewriter.into_result()? {
            let filename = fm.name.to_string();
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

            TxFS::with_buffer(|fs| fs.write(&filename, buf))?;

            modified_files.push(filename.clone());

            report(|| {
                println!("✅  Rewrote references in {}", filename);
            });
        }
    }

    Ok(modified_files)
}
