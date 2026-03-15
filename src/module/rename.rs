use std::path::PathBuf;

use glob::GlobError;

use swc_core::common::{SourceMap, sync::Lrc};

use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use swc_core::{
    common::comments::SingleThreadedComments,
    ecma::{
        ast::*,
        codegen::{Emitter, text_writer::JsWriter},
        parser::{Parser, StringInput, Syntax, TsSyntax, lexer::Lexer},
        visit::VisitMutWith,
    },
};

use crate::ast;

pub fn rename_whitelabel(
    files: &Vec<Result<PathBuf, GlobError>>,
    cm: &Lrc<SourceMap>,
    rename_map: &HashMap<String, String>,
    should_print: bool,
) -> Vec<String> {
    let comments = SingleThreadedComments::default();

    let mut modified_files: Vec<String> = Vec::new();

    for entry in files {
        let Ok(path) = entry else {
            continue;
        };

        let Ok(fm) = cm.load_file(&path) else {
            continue;
        };
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

        let mut wl_rename = ast::rename::WhitelabelRename {
            rename_map,
            has_modified: false,
            should_print,
        };
        program.visit_mut_with(&mut wl_rename);

        if wl_rename.has_modified {
            modified_files.push(path.to_string_lossy().to_string());
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
                let _ = fs::write(&path, code);
            }
        }
    }

    modified_files
}
