pub fn generate(targets: Vec<&String>, default_wl: String) -> String {
    let mut sorted_targets = targets.to_vec();
    sorted_targets.sort();

    let mut index_content = String::new();

    index_content.push_str(
        r#"/* eslint-disable @typescript-eslint/no-require-imports */
// AUTO-GENERATED: DO NOT EDIT
import __current from './determine-whitelabel';
    "#,
    );

    for target in &sorted_targets {
        index_content.push_str(&format!(
            "import type {} from './{}.generated';",
            target, target
        ));
    }
    index_content.push_str(
        format!(
            "export type WhitelabelConfig = InstanceType<typeof {}>;\n",
            default_wl
        )
        .as_str(),
    );

    let mut unions = String::from("export type Variants =");
    for target in &sorted_targets {
        unions.push_str(&format!(
            r#"
        |"{}"
        "#,
            target
        ));
    }

    index_content.push_str(&unions);
    index_content.push_str("\nclass Whitelabel implements Record<Variants, WhitelabelConfig> {\n");
    let mut configs = String::new();
    for target in &sorted_targets {
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
    index_content.push_str("export default new Whitelabel()[__current];\n");
    index_content
}
