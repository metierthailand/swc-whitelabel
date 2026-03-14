use anyhow::Result;
use glob::glob;
use std::fs;
use swc_core::{
    common::{
        SourceMap,
        comments::{Comments, SingleThreadedComments},
        errors::{ColorConfig, Handler},
        sync::Lrc,
    },
    ecma::{
        ast::*,
        parser::{Parser, StringInput, Syntax, TsConfig, lexer::Lexer},
        visit::{Visit, VisitWith},
    },
};

#[derive(Debug)]
struct WhitelabelEntry {
    key: String,
    symbol: String,
    import_path: String,
}

struct WhitelabelCollector<'a> {
    comments: &'a SingleThreadedComments,
    file_path: String,
    pub entries: Vec<WhitelabelEntry>,
    pub errors: Vec<String>,
}

impl<'a> WhitelabelCollector<'a> {
    fn new(comments: &'a SingleThreadedComments, file_path: String) -> Self {
        Self {
            comments,
            file_path,
            entries: vec![],
            errors: vec![],
        }
    }

    /// Robustly scans all leading comments for the whitelabel directive
    fn get_whitelabel_key(&mut self, span: swc_core::common::Span) -> Option<String> {
        let leading_comments = self.comments.get_leading(span.lo)?;
        for comment in leading_comments {
            let text = comment.text.trim();
            if let Some(rest) = text.strip_prefix("whitelabel:") {
                let key = rest.trim().to_string();
                if key.contains('.') {
                    self.errors
                        .push(format!("Forbidden dotted key '{}' found.", key));
                    return None;
                }
                return Some(key);
            }
        }
        None
    }
}

impl<'a> Visit for WhitelabelCollector<'a> {
    // Catch standard `export const` and `export function`
    fn visit_export_decl(&mut self, export: &ExportDecl) {
        if let Some(key) = self.get_whitelabel_key(export.span) {
            match &export.decl {
                Decl::Var(var_decl) => {
                    if let Some(decl) = var_decl.decls.first() {
                        if let Pat::Ident(ident) = &decl.name {
                            self.entries.push(WhitelabelEntry {
                                key,
                                symbol: ident.id.sym.to_string(),
                                import_path: self.file_path.clone(),
                            });
                        }
                    }
                }
                Decl::Fn(fn_decl) => {
                    self.entries.push(WhitelabelEntry {
                        key,
                        symbol: fn_decl.ident.sym.to_string(),
                        import_path: self.file_path.clone(),
                    });
                }
                _ => self.errors.push(format!(
                    "Unsupported export declaration for whitelabel key '{}'",
                    key
                )),
            }
        }
        export.visit_children_with(self);
    }

    // Fail loud on re-exports (e.g., `export { foo as companyName }`)
    fn visit_named_export(&mut self, export: &NamedExport) {
        if self.get_whitelabel_key(export.span).is_some() {
            self.errors.push(format!(
                "File {} contains a whitelabel directive on a named export block. \
                This is not supported in v1. Use direct inline exports.",
                self.file_path
            ));
        }
        export.visit_children_with(self);
    }
}

fn main() -> Result<()> {
    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    let mut all_entries: Vec<WhitelabelEntry> = vec![];
    let mut has_errors = false;

    // Scan for TSX files
    for entry in glob("app/**/*.tsx")
        .unwrap()
        .chain(glob("app/**/*.ts").unwrap())
    {
        let path = entry?;

        // Skip the generated file to avoid infinite loops
        if path.ends_with("whitelabel.generated.tsx") {
            continue;
        }

        let fm = cm.load_file(&path)?;
        let comments = SingleThreadedComments::default();

        let lexer = Lexer::new(
            Syntax::Typescript(TsConfig {
                tsx: true,
                ..Default::default()
            }),
            Default::default(),
            StringInput::from(&*fm),
            Some(&comments),
        );

        let mut parser = Parser::new_from(lexer);
        let module = match parser.parse_module() {
            Ok(m) => m,
            Err(e) => {
                e.into_diagnostic(&handler).emit();
                continue;
            }
        };

        // Format import path (e.g., "src/components/branding.tsx" -> "./components/branding")
        let import_path = format!(
            "./{}",
            path.with_extension("")
                .strip_prefix("src/")
                .unwrap_or(&path)
                .display()
        );

        let mut collector = WhitelabelCollector::new(&comments, import_path);
        module.visit_with(&mut collector);

        if !collector.errors.is_empty() {
            for err in collector.errors {
                eprintln!("❌ Error in {}: {}", path.display(), err);
            }
            has_errors = true;
        }

        all_entries.extend(collector.entries);
    }

    if has_errors {
        anyhow::bail!("Whitelabel extraction failed due to authoring errors.");
    }

    // Generate the output file
    all_entries.sort_by(|a, b| a.key.cmp(&b.key));

    let mut output = String::new();
    output.push_str("// AUTO-GENERATED: DO NOT EDIT\n\n");

    // Generate Imports
    for entry in &all_entries {
        output.push_str(&format!(
            "import {{ {} }} from \"{}\";\n",
            entry.symbol, entry.import_path
        ));
    }

    // Generate Object
    output.push_str("\nconst whitelabel = {\n");
    for entry in &all_entries {
        if entry.key == entry.symbol {
            output.push_str(&format!("  {},\n", entry.key));
        } else {
            output.push_str(&format!("  {}: {},\n", entry.key, entry.symbol));
        }
    }
    output.push_str("};\n\nexport default whitelabel;\n");

    fs::write("app/whitelabel.generated.tsx", output)?;
    println!(
        "✅ Successfully generated src/whitelabel.generated.tsx with {} entries.",
        all_entries.len()
    );

    Ok(())
}
