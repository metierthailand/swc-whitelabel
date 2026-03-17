pub fn generate(default_wl: String) -> String {
    format!(
        r#"
// THIS MODULE IS FOR CUSTOM LOGIC TO DETERMINE CURRENT WHITELABEL
// TODO: possibility for HTTP per-request whitelabel

import type {{ Whitelabel }} from '.'

const currentWhitelabel = (process.env.NEXT_PUBLIC_WHITELABEL as Whitelabel) || '{}'

// ! exported result have to be `satisfies Whitelabel`
export default currentWhitelabel satisfies Whitelabel
"#,
        default_wl
    )
    .to_string()
}
