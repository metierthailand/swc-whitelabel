use std::path::PathBuf;

use swc_core::{
    common::{SourceMap, comments::SingleThreadedComments, sync::Lrc},
    ecma::{
        parser::{Lexer, Parser, StringInput, Syntax, TsSyntax},
        visit::VisitWith,
    },
};
use testing::fixture;
use wl_extractor::{
    ast::collector::{WhitelabelCollector, WhitelabelEntry},
    common::errorable::Errorable,
    config::env::{self, WhitelabelConfig},
    util,
};

#[fixture("tests/fixtures/collector/**/*.tsx")]
fn test_collectors(path: PathBuf) {
    // Setup custom env before run
    match env::init(WhitelabelConfig {
        src: "../collector".to_string(),
        cwd: std::env::current_dir().unwrap(),
        ..Default::default()
    }) {
        Ok(_) => {}
        Err(e) => eprintln!("{:?}", e),
    }

    let cm: Lrc<SourceMap> = Default::default();
    let comments = SingleThreadedComments::default();

    let mut collector = WhitelabelCollector::new(&cm, &comments);
    let fm = cm.load_file(&path).unwrap();

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
            panic!("{:?}", e);
        }
    };

    module.visit_with(&mut collector);

    match collector.into_result() {
        Ok(mut entries) => {
            insta::assert_yaml_snapshot!(
                format!("{}_collector_entries", path.file_name().unwrap().display()),
                entries
                    .iter_mut()
                    .map(|e| {
                        let to_rel = env::with_config(|cfg| {
                            util::compute_relative_import(
                                cfg.cwd.as_path(),
                                PathBuf::from(&e.import_path).as_path(),
                            )
                        });
                        e.import_path = to_rel.unwrap();
                        e
                    })
                    .collect::<Vec<&mut WhitelabelEntry>>()
            );
        }
        Err(e) => {
            insta::assert_yaml_snapshot!(
                format!("{}_collector_errors", path.file_name().unwrap().display()),
                e.to_string()
            );
        }
    }
}
