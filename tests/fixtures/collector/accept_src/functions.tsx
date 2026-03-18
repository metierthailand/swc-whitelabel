/**
 * Shorten form of whitelabel
 * Default target = 'wl'
 */

// whitelabel
export const FN = (a: string) => `Hello! ${a}`;

/**
 * Explicit target
 */

// whitelabel: for=wl
export const FN2 = (a: string) => `Hello! 2 ${a}`;

/**
 * Explicit target: bunny
 */

// whitelabel: for=bunny
export const FN3 = (a: string) => `Hello! 3 ${a}`;

/**
 * Absence of `:` is also allowed
 */

// whitelabel for=bunny
export const FN4 = (a: string) => `Hello! 4 ${a}`;

/**
 * With key overriding
 */

// whitelabel for=bunny, key=FN
export const BUNNY_FN = (a: string) => `Bunny hello to ${a}!`;

/**
 * Key overriding but default target
 */

// whitelabel key=feature_a_FN
export const FN5 = () => <div><h1>Hey</h1></div>;
