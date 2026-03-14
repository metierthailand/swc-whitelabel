use crate::collector;

pub fn generate(entries: &Vec<&collector::WhitelabelEntry>) -> String {
    let mut output = String::new();
    output.push_str("// AUTO-GENERATED: DO NOT EDIT\n\n");

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
        if entry.symbol == entry.key {
            output.push_str(&format!("  {},\n", entry.symbol));
        } else {
            output.push_str(&format!("  {}: {},\n", entry.key, entry.symbol))
        }
    }
    output.push_str("};\n\nexport default whitelabel;\n");

    output
}
