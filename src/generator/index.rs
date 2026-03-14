pub fn generate(index_exports: String, index_configs: String, default_wl: String) -> String {
    let mut index_content = String::new();
    index_content.push_str("// AUTO-GENERATED: DO NOT EDIT\n\n");
    index_content.push_str(&index_exports);
    index_content
        .push_str(format!("export type WhitelabelConfig = typeof {}\n", default_wl).as_str());
    index_content.push_str("\nconst configs: Record<string, WhitelabelConfig> = {\n");
    index_content.push_str(&index_configs);
    index_content.push_str("};\n\n");
    index_content.push_str(
        format!(
            "const currentWhiteLabel = process.env.NEXT_PUBLIC_WHITELABEL || \"{}\";\n",
            default_wl
        )
        .as_str(),
    );
    index_content.push_str("export default configs[currentWhiteLabel];\n");
    index_content
}
