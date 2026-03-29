/**
 * 🎨 WHITELABEL RESOLUTION ENTRY POINT
 * * Unlike the `*.generated.tsx` files in this directory, this file is YOURS to edit!
 * `wl-extractor` generated this as a starter template. You should customize the
 * logic here to match how your application determines the current tenant/brand.
 * * By default, this uses a static, build-time environment variable.
 */

import { type Variants, Whitelabel, type WhitelabelConfig } from "./whitelabel";
import { withFeatureFlags } from "./with-feature-flags";

// --- Default Strategy: Build-time Environment Variable ---
const currentWhitelabel = (() => {
  const variant = "def";
  return new Whitelabel()[variant];
})();

// ! The default export MUST satisfy the `WhitelabelConfig` interface.
// This ensures that wherever you `import whitelabel from '...'`, it is strictly typed.
export default currentWhitelabel satisfies WhitelabelConfig;

export { type Variants, type WhitelabelConfig, withFeatureFlags };

/* * ============================================================================
 * 💡 IDEAS FOR ADVANCED CUSTOM LOGIC
 * ============================================================================
 * * 1. React Context & Hooks (Client-Side Dynamic)
 * ----------------------------------------------------------------------------
 * If your app switches brands dynamically in the browser (e.g., a dropdown),
 * export a hook to access the current config:
 * * export const useWhitelabel = (): WhitelabelConfig => {
 * // e.g., Read from Redux, Zustand, or React Context
 * const currentTenant = useTenantStore((state) => state.tenant);
 * return new Whitelabel()[currentTenant as Variants];
 * };
 * * * 2. Next.js App Router / SSR (Per-Request Whitelabeling)
 * ----------------------------------------------------------------------------
 * If you are hosting multiple tenants on a single Node server, `process.env`
 * won't work. Instead, resolve the brand based on the request headers or URL:
 * * import { headers } from 'next/headers';
 * * export const getServerWhitelabel = (): WhitelabelConfig => {
 * const host = headers().get('host');
 * const variant = host?.includes('brand-a') ? 'brandA' : 'def';
 * return new Whitelabel()[variant as Variants];
 * };
 * ============================================================================
 */
