use crate::{
    common::registry::{WhitelabelRecord, WhitelabelRegistry, WhitelabelSymbol},
    config::env::{self, with_config},
    util::to_rel_import,
};

fn format_doc(entry: &[&WhitelabelRecord]) -> String {
    let default_value = with_config(|cfg| {
        if let Some(e) = entry.iter().find(|e| e.target == cfg.default_target) {
            return format!(
                r#"* @copyright **{}**
              * @default
              * ```tsx
              {}
              * ```"#,
                e.target,
                e.remark
                    .lines()
                    .map(|line| format!("* {}", line))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }

        "".into()
    });

    let implementations = entry
        .iter()
        .filter_map(|e| match &e.symbol {
            WhitelabelSymbol::Symbol { .. } => Some(format!(
                "{{@link {}_{:x} | `{}`}}",
                e.target,
                e.symbol.short_id(),
                e.target,
            )),
            WhitelabelSymbol::Undefined => None,
        })
        .collect::<Vec<_>>();

    format!(
        r#"/**
* ### 🏷️ Available for: {}
{}
*/
"#,
        implementations.join(" | "),
        default_value
    )
}

pub fn generate(registry: &WhitelabelRegistry) -> String {
    let current_dir = env::with_config(|cfg| cfg.cwd.join(&cfg.src).join(&cfg.output_dir));

    let mut index_content = String::new();
    let mut typedef = String::new();

    index_content.push_str(
        r#"/* eslint-disable @typescript-eslint/no-require-imports */
// AUTO-GENERATED: DO NOT EDIT
"#,
    );

    let entries = registry.by_keys();

    for (key, variants) in &entries {
        typedef.push_str(&format_doc(variants));
        typedef.push_str(&format!("{}: ", key));

        for v in variants {
            match &v.symbol {
                WhitelabelSymbol::Symbol {
                    symbol,
                    import_path,
                } => {
                    index_content.push_str(&format!(
                        "import type {{ {} as {}_{:x} }} from '{}';",
                        symbol,
                        v.target,
                        v.symbol.short_id(),
                        to_rel_import(&current_dir, import_path).to_string_lossy()
                    ));
                    typedef.push_str(&format!("| typeof {}_{:x}", v.target, v.symbol.short_id()));
                }
                WhitelabelSymbol::Undefined => {
                    typedef.push_str("| undefined");
                }
            }
        }

        typedef.push_str(",\n");
    }
    index_content.push_str(
        format!(
            r#"export interface WhitelabelConfig {{
            {}
            }};"#,
            typedef
        )
        .as_str(),
    );

    let targets = registry.targets();

    let mut unions = String::from("export type Variants =");
    for target in targets.clone() {
        unions.push_str(&format!(
            r#"
        |"{}"
        "#,
            target
        ));
    }

    index_content.push_str(&unions);
    index_content
        .push_str("\nexport class Whitelabel implements Record<Variants, WhitelabelConfig> {\n");
    let mut configs = String::new();
    for target in targets {
        configs.push_str(&format!(
            r#"public get {}(): WhitelabelConfig {{
              const VariantConfig = require("./{}.generated").default;
              return new VariantConfig();
            }}
            "#,
            // "  {}: require('./{}.generated').default,\n",
            target,
            target
        ));
    }
    index_content.push_str(&configs);
    index_content.push_str("};\n\n");
    index_content
}
