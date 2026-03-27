# Contributing to wl-extractor 👋

First off, thank you for giving consideration to `wl-extractor`! Whether you're looking for something to contribute to, evaluating the architecture, or just improving the documentation, your attention really is appreciated.

This document outlines our official development roadmap and the engineering principles we follow. Since `wl-extractor` is an automated, codebase-wide codemod tool, our guiding philosophy is simple: **We do not offer black-box magic; we provide magic with explicit boundaries.**

We design this tool under the assumption that developers will be disciplined with how they author their codebase. By enforcing strict architectural boundaries (and halting when those boundaries are crossed), we ensure the tool remains 100% safe, predictable, and transparent.

---

## 🗺️ Current Roadmap & Priorities

We ruthlessly prioritize our backlog based on **Blast Radius and Risk Mitigation**. If you want to contribute but aren't sure where to start, check out the milestones below! We are actively targeting **P0** and **P1** issues.

### 🚨 P0: The "Do No Harm" Release (Fixing Corruption, Crashes & Compliance)
**Goal:** Guarantee that running `wl-extractor` will *never* break a working application, and ensure the tool is operationally safe for external teams to adopt.

* [ ] **1. Transactional Codemods:** Stage all file writes, ensure the tool fails fast on errors, and implement rollbacks to prevent partial repository migrations.
* [x] **2. ES Module Circular Dependencies (Runtime App Crash):** Output cyclic-safe code (e.g., getter functions) to prevent `ReferenceError`s from uninitialized lexical bindings.
* [x] **3. Idempotency (Double-writes):** Running the tool twice must yield the exact same file state. 
  * [ ] *Action:* Upgrade injection detection to rely on AST and config validation rather than simple string matching.
* [ ] **4. Stop Silent Omissions:** Log warnings for any unresolved imports or parse failures instead of silently skipping files (e.g., dropping the `Err(_) => continue` pattern).
* [ ] **5. Optional `compilerOptions.paths`:** The tool currently crashes if `paths` is missing in `tsconfig.json`. It must default to an empty map instead of panicking.
* [x] **6. Graceful Unresolved Imports:** Unresolved imports should log a warning and be skipped, not crash the binary.
* [x] **7. JSX Closing Tags (Syntax Corruption):** Rewrite `</BrandAHeader>` to `</whitelabel.HeroHeader>` to preserve React AST integrity.

### 🛑 P1: The "Adoption Blocker" Release (Stability & Predictability)
**Goal:** Ensure the tool runs smoothly on standard, messy real-world repositories without throwing Rust `panic!` traces and behaves predictably.

* [x] **8. Halt on Validation Errors & Fix CLI Exit Codes:** If the collector finds an invalid directive, the tool **must fail with a non-zero exit code** and halt. "Partial success" is unacceptable.
* [x] **9. Eradicate `unwrap/expect/panic` in `run.rs`:** Convert initialization, glob parsing, and file I/O errors into contextual `anyhow::Result` user-facing error messages.
  * [ ] *Action:* Eliminate remaining `todo!()` macros in the execution hot path.
* [ ] **10. Secure Path Resolution:** Validate that user-provided configuration patterns cannot escape the intended source root directory.
* [ ] **11. Define Public Contract:** Establish a versioned configuration schema and a clear compatibility matrix for users.
* [ ] **12. Comprehensive E2E Tests:** Expand `tests/integration_test.rs` to cover alias-heavy imports, malformed configs, self-closing JSX, multi-declarator exports, and syntax errors.

### 🟡 P2: The "Data Fidelity & Polish" Release
**Goal:** Guarantee perfect capture of developer intent, establish performance baselines, and formalize the release process.

* [x] **13. Duplicate Key Detection:** Fail loudly if two exports target the same `key` for the same brand.
* [x] **14. Enterprise Optionality (`optional` & `null as never`):** Inject `null as never` for keys that aren't implemented in the default whitelabel, forcing TypeScript compilation errors over runtime crashes.
* [x] **15. Implement CFG Grammar for Directives:** Allow flexible, shuffled comma-separated parameters in the `// whitelabel:` comments.
* [ ] **16. Multi-Declarator Exports:** Support `export const A = 1, B = 2;` instead of silently dropping `B` via `decls.first()`.
* [ ] **17. Formalize Release Engineering:** Adopt semantic versioning, utilize GitHub tags, and maintain a structured changelog.
* [ ] **18. Establish Performance Baselines:** Benchmark the tool against large repository fixtures to track runtime and memory footprint scaling.

### 🛠️ P3: The "Engineering Rigor" Release (Tech Debt & CI)
**Goal:** Make the codebase a joy to contribute to, secure, and mathematically proven to work.

* [x] **19. Destroy the Global `OnceLock`:** Refactor `src/config/config.rs`. Passing a `&WhitelabelConfig` context down the pipeline eliminates the testability trap.
* [x] **20. CI/CD Hardening:** Include `cargo fmt --check`, `cargo clippy -- -D warnings`, and a release build verification.
  * [ ] *Action:* Add automated CI gates to enforce idempotency (zero diffs on second run) and strict non-zero exit codes on failure.
* [ ] **21. Automate Security Checks:** Implement dependency audits and establish allow/deny policies for third-party crates.
* [ ] **22. Add Governance Documentation:** Create a `CODE_OF_CONDUCT.md`, `SECURITY.md`, and standardized issue/PR templates for contributors.

### 🎨 P4: The "Documentation" Release
**Goal:** Align the README promises with the actual code behavior and improve DX.

* [ ] **23. Clarify Resolver Semantics:** Update the README to explicitly state that the resolver supports `paths` and wildcard matching, but does *not* claim 100% Node.js parity.
* [ ] **24. Document Brand Selection Strategy:** Frame the `NEXT_PUBLIC_WHITELABEL` generated code as a "starter strategy".
* [ ] **25. Clean up `package.json`:** Either explain why it's there (for fixture linting) or delete it to keep the Rust repo focused.

---

## 💻 Getting Started

To get a local development environment running:

1. **Clone the repository:**
   ```bash
   git clone https://github.com/metierthailand/swc-whitelabel.git
   cd swc-whitelabel
   ```
2. **Run the test suite:**
   We use [`insta`](https://insta.rs/docs/quickstart/) for snapshot testing. Ensure all baseline tests pass before starting your work.
   ```bash
   cargo test
   ```
3. **Review snapshots (if modifying AST visitors):**
   If you change the structure of the codemod, you will need to accept the new AST snapshots.
   ```bash
   cargo insta review
   ```
