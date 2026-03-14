pub fn generate(index_exports: String, index_configs: String) -> String {
    let mut index_content = String::new();
    index_content.push_str("// AUTO-GENERATED: DO NOT EDIT\n\n");
    index_content.push_str(&index_exports);
    index_content.push_str("\nconst configs: Record<string, any> = {\n");
    index_content.push_str(&index_configs);
    index_content.push_str("};\n\n");
    index_content
        .push_str("const currentBrand = process.env.NEXT_PUBLIC_BRAND || \"trivacafe\";\n");
    index_content.push_str("export default configs[currentBrand];\n");
    index_content
}
