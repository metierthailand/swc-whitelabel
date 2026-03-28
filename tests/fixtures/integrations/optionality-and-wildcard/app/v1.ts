// whitelabel for 'v1', as 'FLAG_ENABLE_FEATURE_A'
export const V1_FLAG_ENABLE_FEATURE_A = false;

// whitelabel optional, for 'v1'
export const V1_ONLY_FEATURE = "Only present in def";

//  We intentionally do NOT implement GLOBAL_FEATURE for v1.
//  Because it was marked with '*', the registry will automatically
//  point v1's GLOBAL_FEATURE to the `def.ts` implementation!
