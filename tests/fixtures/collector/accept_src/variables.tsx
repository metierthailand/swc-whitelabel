/**
 * Shorten form of whitelabel
 * Default target = '__def__'
 */

// whitelabel
export const TITLE = "ELTIT";

/**
 * Explicit target
 */

// whitelabel: for=wl
export const TITLE_1 = "1_ELTIT";

/**
 * Explicit target: bunny
 */

// whitelabel: for=bunny
export const TITLE_2 = "2_ELTIT";

/**
 * Absence of `:` is also allowed
 */

// whitelabel for=bunny
export const TITLE_3 = "3_ELTIT";

/**
 * Natural marker
 */
// whitelabel for wl
export const CFG_TITLE_3 = "3_ELTIT_GFC";

/**
 * With key overriding
 */

// whitelabel for=bunny, key=TITLE
export const TITLE_4 = "4_ELTIT";

/**
 * Or `as`
 */

// whitelabel for=bunny, as TITLE
export const CFG_TITLE_4 = "4_ELTIT_GFC";

/**
 * Or
 */

// whitelabel for bunny, as TITLE
export const CFG_TITLE_4_1 = "1_4_ELTIT_GFC";

/**
 * Or
 */

// whitelabel for 'wl' as 'TITLE'
export const CFG_TITLE_4_2 = "2_4_ELTIT_GFC";

/**
 * Or
 * (Not recommended, hard to read)
 */

// whitelabel for wl as TITLE
export const CFG_TITLE_4_3 = "3_4_ELTIT_GFC";

/**
 * Key overriding but default target
 */

// whitelabel key=TITLE
export const TITLE_5 = "5_ELTIT";

/**
 * Multiple targets are allowed
 */

// whitelabel for 'a', for 'b'
export const MULTIPLE_TARGETS = ["a", "b"];
