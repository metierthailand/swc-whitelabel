pub fn generate(targets: Vec<&String>, default_wl: String) -> String {
    let mut imports = String::new();
    let mut configs = String::new();
    let mut unions = String::from("export type Whitelabel =");

    for target in &targets {
        imports.push_str(&format!(
            "import {} from \"./{}.generated\";\n",
            target, target
        ));
        configs.push_str(&format!("  {},\n", target));
        unions.push_str(&format!(
            r#"
        |"{}"
        "#,
            target
        ));
    }

    let mut index_content = String::new();

    index_content.push_str("// AUTO-GENERATED: DO NOT EDIT\n\n");
    index_content.push_str("import __current from './determine-whitelabel';\n");
    index_content.push_str(&imports);
    index_content
        .push_str(format!("export type WhitelabelConfig = typeof {}\n", default_wl).as_str());
    index_content.push_str(&unions);
    index_content.push_str("\nconst configs: Record<Whitelabel, WhitelabelConfig> = {\n");
    index_content.push_str(&configs);
    index_content.push_str("};\n\n");
    index_content.push_str("export default configs[__current];\n");
    index_content
}
