pub fn generate(default_wl: String) -> String {
    format!(
        r#"
// THIS MODULE IS FOR CUSTOM LOGIC TO DETERMINE CURRENT WHITELABEL
// TODO: possibility for HTTP per-request whitelabel

import type {{ Variants }} from '.'

const currentWhitelabel = (process.env.NEXT_PUBLIC_WHITELABEL as Variants) || '{}'

// ! exported result must be `satisfies Variants`
export default currentWhitelabel satisfies Variants
"#,
        default_wl
    )
    .to_string()
}
