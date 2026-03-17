use crate::ast::collector::{self, WhitelabelEntry};

fn format_doc(entry: &WhitelabelEntry) -> String {
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
        entry.target.clone().unwrap_or_default(),
        entry.import_path,
        entry.symbol,
        entry
            ._experiment_remark
            .lines()
            .map(|line| format!("* {}", line))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

pub fn generate(entries: &Vec<collector::WhitelabelEntry>, is_default: bool) -> String {
    let mut output = String::new();
    output.push_str(if !is_default {
        "// AUTO-GENERATED: DO NOT EDIT\n\nimport type { WhitelabelConfig } from '.';\n"
    } else {
        "// AUTO-GENERATED: DO NOT EDIT\n\n"
    });

    let mut sorted_entries = entries.clone();
    sorted_entries.sort_by(|a, b| a.key.cmp(&b.key));

    for entry in &sorted_entries {
        output.push_str(&format!(
            "import {{ {} }} from \"{}\";\n",
            entry.symbol, entry.import_path
        ));
    }

    output.push_str("\nconst whitelabel = {\n");
    for entry in &sorted_entries {
        output.push_str(&format_doc(entry));
        if entry.symbol == entry.key {
            output.push_str(&format!("  {},\n", entry.symbol));
        } else {
            output.push_str(&format!("  {}: {},\n", entry.key, entry.symbol))
        }
    }
    output.push_str(if !is_default {
        "} satisfies WhitelabelConfig;\n\nexport default whitelabel;\n"
    } else {
        "};\n\nexport default whitelabel;\n"
    });

    output
}
