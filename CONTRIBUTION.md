### 🚨 P0: The "Do No Harm" Release (Fixing Corruption & Crashes)
**Goal:** Guarantee that running `wl-extractor` will *never* break a working application, either at compile-time or runtime.

* **1. ✅ ES Module Circular Dependencies (Runtime App Crash):** If our codemod forces a top-level read of an uninitialized lexical binding, the ES Module loader throws a `ReferenceError`. We must output cyclic-safe code (e.g., getter functions).
* **2. Idempotency (Double-writes):** Running the tool twice must yield the exact same file state. No duplicate `import whitelabel from...` injections.
* **3. Optional `compilerOptions.paths`:** Many valid TypeScript projects don't use `paths`. The tool currently crashes via `expect("Failed to load tsconfig.json")` if this is missing. It must default to an empty map instead of panicking.
* **4. Graceful Unresolved Imports:** Right now, one bad/unresolvable import causes an `expect()` panic that aborts the entire codemod. Unresolved imports should log a warning and be skipped, not crash the binary.
* **5. JSX Closing Tags (Syntax Corruption):** The codemod must rewrite `</BrandAHeader>` to `</whitelabel.HeroHeader>`, otherwise the React AST is physically broken.

### 🛑 P1: The "Adoption Blocker" Release (Fixing Panics)
**Goal:** Ensure the tool runs smoothly on standard, messy real-world repositories without throwing Rust `panic!` traces.

* **6. Halt on Validation Errors & Fix CLI Exit Codes:** Stop swallowing errors in `main.rs`. If the collector finds an invalid directive, the tool **must fail with a non-zero exit code** and halt before the codemod phase. "Partial success" is unacceptable for codebase-wide migrations.
* **7. Eradicate `unwrap/expect/panic` in `run.rs`:** Convert all initialization, glob parsing, and file I/O errors into contextual `anyhow::Result` user-facing error messages. 

### 🟡 P2: The "Data Fidelity" Release (Fixing Silent Omissions)
**Goal:** Guarantee that if a developer writes valid code or configurations, the tool captures it perfectly and warns them if they make a logical mistake.

* **8. Duplicate Key Detection:** Fail loudly if two exports target the same `key` for the same brand, preventing silent HashMap overwrites and missing logic.
* **9. Log Silent Parse Skips:** Currently, `Err(_) => continue` silently skips unparseable files. Emit a console warning so users know a file wasn't transformed.
* **10. Enterprise Optionality (`optional` & `null as never`):** Inject `null as never` for keys that aren't implemented in the default whitelabel, forcing TypeScript to catch missing tenant features at compile-time rather than crashing at runtime.
* **11. Implement CFG Grammar for Directives:** Allow flexible, shuffled comma-separated parameters in the `// whitelabel:` comments.

### 🛠️ P3: The "Engineering Rigor" Release (Tech Debt & CI)
**Goal:** Make the codebase a joy to contribute to and mathematically proven to work.

* **12. ✅ Destroy the Global `OnceLock`:** Refactor `src/config/config.rs`. Passing a `&WhitelabelConfig` context down the pipeline eliminates the testability trap and allows for concurrent test runners.
* **13. CI/CD Hardening:** Update GitHub Actions to include `cargo fmt --check`, `cargo clippy -- -D warnings`, and a release build verification.
* **14. Comprehensive E2E Tests:** Expand `tests/integration_test.rs` to cover alias-heavy imports, malformed configs, self-closing JSX, and unresolved imports.

### 🎨 P4: The "Documentation & Polish" Release
**Goal:** Align the README promises with the actual code behavior and improve DX.

* **15. Clarify Resolver Semantics:** Update the README to explicitly state that the resolver supports `paths` and wildcard matching, but does *not* claim 100% Node.js/TypeScript parity (like missing `baseUrl`).
* **16. Document Brand Selection Strategy:** Frame the `NEXT_PUBLIC_WHITELABEL` generated code as a "starter strategy" that users can customize for SSR/multi-tenant routing.
* **17. Clean up `package.json`:** Either explain why it's there (for fixture linting) or delete it to keep the Rust repo focused.
* **18. Multi-Declarator Exports:** Support `export const A = 1, B = 2;` instead of silently dropping `B` via `decls.first()`.
