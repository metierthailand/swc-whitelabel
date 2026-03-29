#[cfg(test)]
mod tests {
    use biome_formatter::{IndentStyle, IndentWidth, LineWidth};
    use biome_js_formatter::context::BracketSameLine;
    use biome_js_formatter::{context::JsFormatOptions, format_node};
    use biome_js_parser::{JsParserOptions, parse};
    use biome_js_syntax::JsFileSource;
    use insta::assert_snapshot;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;
    use testing::fixture;
    use walkdir::WalkDir;
    // use wl_extractor::exec::run;

    // --- 1. The Isolated Workspace Helper ---

    struct TestWorkspace {
        dir: TempDir,
    }

    impl TestWorkspace {
        /// Creates a temp directory and copies the fixture into it
        pub fn from_fixture(fixture_root: &Path) -> Self {
            let temp = TempDir::new().expect("Failed to create temp dir");

            for entry in WalkDir::new(fixture_root) {
                let entry = entry.unwrap();
                let relative_path = entry.path().strip_prefix(fixture_root).unwrap();
                let target_path = temp.path().join(relative_path);

                if entry.file_type().is_dir() {
                    fs::create_dir_all(&target_path).unwrap();
                } else {
                    fs::copy(entry.path(), &target_path).unwrap();
                }
            }

            Self { dir: temp }
        }

        pub fn path(&self) -> &Path {
            self.dir.path()
        }

        /// Gathers all file contents in the workspace for snapshotting
        pub fn snapshot_results(&self) -> String {
            let mut results = String::new();

            for entry in WalkDir::new(self.path()).sort_by_file_name() {
                let entry = entry.unwrap();
                if entry.file_type().is_file() {
                    let rel_path = entry.path().strip_prefix(self.path()).unwrap();
                    let content = fs::read_to_string(entry.path()).unwrap();

                    let prettified = match rel_path.extension() {
                        Some(o_str) => {
                            if o_str == "ts" || o_str == "tsx" {
                                //  Parse the code into Biome's Syntax Tree
                                let parsed = parse(
                                    &content,
                                    JsFileSource::tsx(),
                                    JsParserOptions::default(),
                                );

                                let format_options = JsFormatOptions::new(JsFileSource::tsx())
                                    .with_indent_style(IndentStyle::Space)
                                    .with_indent_width(IndentWidth::from(2))
                                    .with_line_width(LineWidth::try_from(LineWidth::MAX).unwrap())
                                    .with_bracket_same_line(BracketSameLine::from(true));

                                //  Run the formatter on the parsed syntax tree
                                let formatted = format_node(format_options, &parsed.syntax())
                                    .expect("Failed to format the AST");

                                //  Print the formatted tree back into a standard Rust String
                                let printed = formatted.print().expect("Failed to print code");

                                let code = printed.into_code();
                                // let _ = fs::write(entry.path(), &code);
                                code
                            } else {
                                content
                            }
                        }
                        None => content,
                    };

                    // Format with markdown for beautiful snapshot diffs
                    results.push_str(&format!(
                        "--- {} ---\n{}\n\n",
                        rel_path.display(),
                        prettified
                    ));
                }
            }
            results
        }
    }

    // --- 2. The Auto-Generated Fixture Runner ---

    // We target the config file as the "anchor" for each test fixture folder
    #[fixture("tests/fixtures/integrations/**/whitelabel.config.json")]
    #[fixture("tests/fixtures/integrations/missing-configs/test")]
    fn test_whitelabel_extraction(config_path: PathBuf) {
        // 1. Get the root folder of this specific test case (e.g., "01_basic_extraction")
        let fixture_root = config_path.parent().unwrap();
        let fixture_name = fixture_root.file_name().unwrap().to_string_lossy();

        // 2. Clone the fixture into a safe, isolated temporary directory
        let workspace = TestWorkspace::from_fixture(fixture_root);

        // 3. RUN YOUR TOOL
        // Note: You will need to expose your main application logic as a
        // library function that accepts a working directory, rather than
        // relying on the global `env::current_dir()`.

        if let Err(e) = wl_extractor::run::run(Some(workspace.path().to_path_buf())) {
            assert_snapshot!(fixture_name.to_string() + "-error", e);
            return;
        }

        // FIXME: assertion should fails if there is an error snapshot.

        // 4. Assert the Results!
        // This takes everything in the temp folder and compares it to the saved snapshot
        let final_output = workspace.snapshot_results();

        // We pass `fixture_name` so insta names the snapshot file correctly!
        assert_snapshot!(fixture_name.to_string(), final_output);

        // Idempotent test

        let _ = wl_extractor::run::run(Some(workspace.path().to_path_buf()));
        let idempt_output = workspace.snapshot_results();
        assert_snapshot!(fixture_name.to_string(), idempt_output);
    }
}
