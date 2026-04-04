#[cfg(test)]
mod tests {
    use anyhow::Result;
    use biome_formatter::{IndentStyle, IndentWidth, LineWidth};
    use biome_js_formatter::context::BracketSameLine;
    use biome_js_formatter::{context::JsFormatOptions, format_node};
    use biome_js_parser::{JsParserOptions, parse};
    use biome_js_syntax::JsFileSource;
    use insta::assert_snapshot;
    use std::path::{Path, PathBuf};
    use std::{fs, io};
    use tempfile::TempDir;
    use testing::fixture;
    use walkdir::WalkDir;
    use wl_extractor::config::env::WhitelabelConfig;
    use wl_extractor::run::RunOptions;

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

        pub fn file_snapshot(&self, file_name: String) -> String {
            let mut full_path = self.path().to_path_buf();
            full_path.push(file_name);
            return fs::read_to_string(full_path.as_path()).unwrap();
        }

        pub fn remove_file(&self, file_name: String) -> io::Result<()> {
            let mut full_path = self.path().to_path_buf();
            full_path.push(file_name);
            return fs::remove_file(full_path.as_path());
        }
    }

    impl AsRef<TestWorkspace> for TestWorkspace {
        fn as_ref(&self) -> &TestWorkspace {
            return &self;
        }
    }

    #[derive(Clone)]
    struct TestWorkspaceConfig {
        cwd: PathBuf,
        with_manifest: bool,
    }

    impl From<&TestWorkspace> for TestWorkspaceConfig {
        fn from(value: &TestWorkspace) -> Self {
            Self {
                cwd: value.path().to_path_buf(),
                with_manifest: false,
            }
        }
    }

    impl RunOptions for TestWorkspaceConfig {
        fn provide_config(&self) -> Result<WhitelabelConfig> {
            let cwd = self.cwd.clone();
            let cfg_file = cwd.join("whitelabel.config.json");

            let mut config = if let Ok(config_str) = fs::read_to_string(&cfg_file) {
                serde_json::from_str::<WhitelabelConfig>(&config_str)?
            } else {
                WhitelabelConfig::default()
            };

            config.tsconfig = cwd.join(&config.tsconfig).to_string_lossy().to_string();
            config.with_manifest = self.with_manifest;

            let temp_dir_as_cwd = WhitelabelConfig { cwd, ..config };

            Ok(temp_dir_as_cwd)
        }
    }

    impl TestWorkspaceConfig {
        fn with_manifest(&self) -> TestWorkspaceConfig {
            TestWorkspaceConfig {
                with_manifest: true,
                ..self.clone()
            }
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
        let test_config: TestWorkspaceConfig = workspace.as_ref().into();

        // 3. RUN YOUR TOOL
        // Note: You will need to expose your main application logic as a
        // library function that accepts a working directory, rather than
        // relying on the global `env::current_dir()`.

        let x = test_config.with_manifest().clone();

        if let Err(e) = wl_extractor::run::run(x) {
            assert_snapshot!(fixture_name.to_string() + "-error", e);
            return;
        }

        let m = format!(
            "{}{}/manifest.json",
            test_config.provide_config().unwrap().src,
            test_config.provide_config().unwrap().output_dir
        );
        let manifest_output = workspace.file_snapshot(m.clone());
        let _ = workspace.remove_file(m);

        assert_snapshot!(fixture_name.to_string() + "-manifest", manifest_output);

        let _ = wl_extractor::run::run(test_config.clone());
        let final_output = workspace.snapshot_results();
        assert_snapshot!(fixture_name.to_string(), final_output);

        // Idempotent test
        let _ = wl_extractor::run::run(test_config.clone());
        let idempt_output = workspace.snapshot_results();
        assert_snapshot!(fixture_name.to_string(), idempt_output);
    }
}
