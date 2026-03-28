use crate::{
    common::registry::{WhitelabelRecord, WhitelabelSymbol},
    config::env,
    util::to_rel_import,
};

pub fn generate(entries: Vec<&WhitelabelRecord>) -> String {
    let current_dir = env::with_config(|cfg| cfg.cwd.join(&cfg.src).join(&cfg.output_dir));

    let mut output = String::new();
    output.push_str(
        r#"/* eslint-disable @typescript-eslint/no-require-imports */

// AUTO-GENERATED: DO NOT EDIT

import type { WhitelabelConfig } from './whitelabel';"#,
    );

    let mut sorted = entries;

    sorted.sort_by(|a, b| a.key.cmp(&b.key));

    output.push_str("\nexport class whitelabel implements WhitelabelConfig {\n");
    for entry in sorted {
        let getter = match &entry.symbol {
            WhitelabelSymbol::Symbol {
                symbol,
                import_path,
            } => format!(
                r#"public get {}(): WhitelabelConfig['{}'] {{
                    return require('{}').{}
                  }}
              "#,
                entry.key,
                entry.key,
                to_rel_import(&current_dir, import_path).to_string_lossy(),
                symbol
            ),
            WhitelabelSymbol::Undefined => format!(
                r#"public get {}(): undefined {{
                    return undefined
                  }}
              "#,
                entry.key,
            ),
        };
        output.push_str(&getter);
    }

    output.push_str("};\n\nexport default whitelabel;\n");

    output
}
