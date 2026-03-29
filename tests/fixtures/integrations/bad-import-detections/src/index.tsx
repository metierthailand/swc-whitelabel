// override named import 'whitelabel' at difference path should be rejected.
import { withFeatureFlags as whitelabel } from "./whitelabel/with-feature-flags";

// whitelabel
export const replacable_title = "TITLE";

console.log(replacable_title);

// whitelabel
export const flag_is_enable = true;

export default whitelabel(
  () => <h1>Something</h1>,
  (wl) => wl.flag_is_enable,
);
