// This violates the CFG (Optional + Wildcard Conflict)
// A feature cannot be simultaneously forced everywhere (*) and marked as optional.

// whitelabel optional, *
export const INVALID_WildcardOrOptionalConflict = "I will crash the parser";
