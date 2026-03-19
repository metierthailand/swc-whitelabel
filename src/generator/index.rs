pub fn generate(targets: Vec<&String>, default_wl: String) -> String {
    let mut configs = String::new();
    let mut unions = String::from("export type Whitelabel =");

    let mut sorted_targets = targets.to_vec();
    sorted_targets.sort();

    for target in sorted_targets {
        configs.push_str(&format!(
            "  {}: require('./{}.generated').default,\n",
            target, target
        ));
        unions.push_str(&format!(
            r#"
        |"{}"
        "#,
            target
        ));
    }

    let mut index_content = String::new();

    index_content.push_str(
        r#"/* eslint-disable @typescript-eslint/no-require-imports */
// AUTO-GENERATED: DO NOT EDIT
import __current from './determine-whitelabel';
    "#,
    );
    index_content.push_str(&format!(
        "import type {} from \"./{}.generated\";\n",
        default_wl, default_wl
    ));
    index_content
        .push_str(format!("export type WhitelabelConfig = typeof {}\n", default_wl).as_str());
    index_content.push_str(&unions);
    index_content.push_str("\nconst configs: Record<Whitelabel, WhitelabelConfig> = {\n");
    index_content.push_str(&configs);
    index_content.push_str("};\n\n");
    index_content.push_str("export default configs[__current];\n");
    index_content
}
