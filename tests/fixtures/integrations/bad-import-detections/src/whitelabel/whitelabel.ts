/* eslint-disable @typescript-eslint/no-require-imports */
// AUTO-GENERATED: DO NOT EDIT
import type { flag_is_enable as def_2389ebae0dd1c5f4 } from "../index";
export interface WhitelabelConfig {
  /**
   * ### 🏷️ Available for: {@link def_2389ebae0dd1c5f4 | `def`}
   * @copyright **def**
   * @default
   * ```tsx
   * true
   * ```
   */
  flag_is_enable: typeof def_2389ebae0dd1c5f4;
}
export type Variants = "def";

export class Whitelabel implements Record<Variants, WhitelabelConfig> {
  public get def(): WhitelabelConfig {
    const VariantConfig = require("./def.generated").default;
    return new VariantConfig();
  }
}
