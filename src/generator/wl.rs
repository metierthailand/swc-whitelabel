use std::path::PathBuf;

use crate::{
    ast::collector::{self, WhitelabelEntry},
    config::config,
    util,
};

fn to_rel_import(current_dir: &PathBuf, entry: &WhitelabelEntry) -> PathBuf {
    let relative_import = match util::compute_relative_import(
        current_dir,
        PathBuf::from(&entry.import_path).as_path(),
    ) {
        Some(s) => PathBuf::from(s).with_extension(""),
        None => todo!(),
    };
    relative_import
}

fn format_doc(entry: &WhitelabelEntry, current_dir: &PathBuf) -> String {
    format!(
        r#"/**
* ### 🏷️ Tenant: `{}`
*
* **from `{}`**
*
* Go to {{@link {} | implementation}}.
* @default
* ```tsx
{}
* ```
*
*/
"#,
        entry.target.as_deref().unwrap_or_default(),
        to_rel_import(&current_dir, entry).to_string_lossy(),
        entry.symbol,
        entry
            ._experiment_remark
            .lines()
            .map(|line| format!("* {}", line))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

pub fn generate(entries: &Vec<&collector::WhitelabelEntry>, is_default: bool) -> String {
    let mut output = String::new();
    output.push_str(if !is_default {
        "/* eslint-disable @typescript-eslint/no-require-imports */\n\n// AUTO-GENERATED: DO NOT EDIT\n\nimport type { WhitelabelConfig } from '.';\n"
    } else {
        "/* eslint-disable @typescript-eslint/no-require-imports */\n\n// AUTO-GENERATED: DO NOT EDIT\n\n"
    });

    let mut sorted_entries: Vec<&WhitelabelEntry> = entries.to_vec();
    sorted_entries.sort_by_key(|e| &e.key);

    let current_dir = config::with_config(|cfg| {
        let mut current_dir = cfg.cwd.clone();
        current_dir.push(&cfg.src);
        current_dir.push(&cfg.output_dir);

        current_dir
    });

    for entry in &sorted_entries {
        let relative_import = to_rel_import(&current_dir, entry);
        output.push_str(&format!(
            "import type {{ {} }} from \"{}\";\n",
            entry.symbol,
            relative_import.to_string_lossy()
        ));
    }

    output.push_str(if !is_default {
        "\nexport class whitelabel implements WhitelabelConfig {\n"
    } else {
        "\nexport class whitelabel {\n"
    });
    for entry in &sorted_entries {
        let relative_import = to_rel_import(&current_dir, entry);

        output.push_str(&format_doc(entry, &current_dir));
        output.push_str(&format!(
            r#"public get {}(): typeof {} {{
                return require('{}').{}
              }}
          "#,
            entry.key,
            entry.symbol,
            relative_import.to_string_lossy(),
            entry.symbol
        ));
    }

    output.push_str("};\n\nexport default whitelabel;\n");

    output
}
