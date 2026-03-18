use std::path::PathBuf;

use swc_core::{
    common::{SourceMap, comments::SingleThreadedComments, sync::Lrc},
    ecma::{
        parser::{Lexer, Parser, StringInput, Syntax, TsSyntax},
        visit::VisitWith,
    },
};
use testing::fixture;
use wl_extractor::ast::collector::WhitelabelCollector;

#[fixture("tests/fixtures/collector/**/*.tsx")]
fn test_whitelabel_extraction(path: PathBuf) {
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

    insta::assert_yaml_snapshot!(
        format!("{}_collector_errors", path.file_name().unwrap().display()),
        collector.errors
    );

    insta::assert_yaml_snapshot!(
        format!("{}_collector_entries", path.file_name().unwrap().display()),
        collector.entries
    );
}
