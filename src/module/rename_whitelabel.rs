use anyhow::{Result, anyhow};
use swc_core::common::{SourceFile, SourceMap, sync::Lrc};

use std::collections::HashMap;
use swc_core::{
    common::comments::SingleThreadedComments,
    ecma::{
        ast::*,
        codegen::{Emitter, text_writer::JsWriter},
        parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer},
        visit::VisitMutWith,
    },
};

use crate::{ast, util::transactional::TxFS};

#[allow(unused)]
pub fn exec(cm: &Lrc<SourceMap>, rename_map: &HashMap<String, String>) -> Result<Vec<String>> {
    let comments = SingleThreadedComments::default();
    let mut modified_files: Vec<String> = Vec::new();
    let files: Vec<Lrc<SourceFile>> = {
        // 🔒 Acquire the lock
        let guard = cm.files();

        // Clone the Arcs into a new Vec
        guard.iter().cloned().collect()
    };

    for fm in files {
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

        let mut wl_rename = ast::rename::WhitelabelRename {
            rename_map,
            has_modified: false,
        };
        program.visit_mut_with(&mut wl_rename);

        if wl_rename.has_modified {
            modified_files.push(fm.name.to_string());
            let mut buf = vec![];
            let mut emitter = Emitter {
                cfg: swc_core::ecma::codegen::Config::default()
                    .with_target(EsVersion::Es2022)
                    .with_omit_last_semi(true),
                cm: cm.clone(),
                comments: Some(&comments),
                wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
            };
            let _ = emitter.emit_program(&program);
            if let Ok(code) = String::from_utf8(buf) {
                TxFS::with_buffer(|fs| fs.write(fm.name.to_string(), code))?;
            }
        }
    }

    Ok(modified_files)
}
