import { withFeatureFlags } from "./whitelabel";

// whitelabel
export const replacable_title = "TITLE";

console.log(replacable_title);

// whitelabel
export const flag_is_enable = true;

export default withFeatureFlags(
  () => <h1>Something</h1>,
  (wl) => wl.flag_is_enable,
);
