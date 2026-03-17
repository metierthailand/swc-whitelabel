# 🏷️ Whitelabel Extractor (wl-extractor)

Welcome to the **Whitelabel Extractor**! This is a lightning-fast, Rust-based AST transformation tool built on top of [SWC](https://swc.rs/).

In a multi-tenant application, we often have different configurations, strings, or UI components for different brands (targets). Instead of cluttering the codebase with `if (brand === 'X')`, this tool allows developers to write standard localized variables, and it handles the heavy lifting of extracting them, generating a registry, and rewriting the codebase to use them dynamically.

## 🚀 Features

- ⚡️ **Blazing Fast**: Parses and transforms thousands of TS/TSX files in milliseconds.
- 🧠 **AST-Aware**: Understands object shorthands, JSX, and variable shadowing.
- 🔗 **Smart Pathing**: Respects `tsconfig.json` aliases (`paths`) and generates bulletproof relative imports.
- 🪄 **Auto-Renaming**: If you change a whitelabel key, the tool automatically updates all references across the repo.

---

## ⚙️ Configuration

Before running the tool, create a `whitelabel.config.json` file in the root of your project:

```json
{
  "src": "./src",
  "patterns": ["/**/*.tsx", "/**/*.ts"],
  "output_dir": "/app/whitelabel",
  "default_target": "defaultBrand",
  "tsconfig": "./tsconfig.json"
}
```

---

## 🛠️ How to Use (Authoring Guide)

### 1. Mark your Exports

To mark a variable, function, or component for extraction, simply add a magic comment directly above the `export` statement.

The directive format is: `// whitelabel: for=<target>, key=<custom_key>`

- **`for`**: (Optional) The specific brand/tenant this code applies to. If omitted, it falls back to the `default_target` from your config. You can specify multiple targets separated by commas.
- **`key`**: (Optional) A custom key for the registry. If omitted, the tool uses the variable's original name.

**Example:**

```tsx
// src/components/Hero/index.tsx

// whitelabel: for=brandA, key=HeroHeader
export const BrandAHeader = () => <h1>Welcome to Brand A</h1>;

// whitelabel: for=brandB, key=HeroHeader
export const BrandBHeader = () => <h1>Welcome to Brand B</h1>;

// whitelabel
export const fallbackDescription = "Standard description";
```

### 2. Run the Extractor

Execute the CLI tool in your terminal:

```bash
cargo build --release && target/release/wl-extractor
```

_(For CI/CD or piping, use `--file-name-only` to suppress human-readable logs.)_

### 3. See the Magic (The Result)

The tool will automatically generate a registry in your `output_dir` (e.g., `src/app/whitelabel`) containing:

1.  `brandA.generated.tsx` and `brandB.generated.tsx`
2.  An `index.ts` that unites them into a strictly typed `WhitelabelConfig`.
3.  A `determine-whitelabel.ts` file that uses `process.env.NEXT_PUBLIC_WHITELABEL` to select the current brand.

**Most importantly, it rewrites your local usages.** If you imported `BrandAHeader` somewhere else in your code, the tool transforms it:

**Before:**

```tsx
import { BrandAHeader, fallbackDescription } from "./components/Hero";

export const Page = () => {
  return (
    <div>
      <BrandAHeader />
      <p>{fallbackDescription}</p>
    </div>
  );
};
```

**After:**

```tsx
import whitelabel from "../app/whitelabel"; // Auto-injected relative path

export const Page = () => {
  return (
    <div>
      <whitelabel.HeroHeader />
      <p>{whitelabel.fallbackDescription}</p>
    </div>
  );
};
```

---

## 🧠 How It Works (Under the Hood)

The extractor operates in three distinct phases using `swc_core` for robust Abstract Syntax Tree (AST) manipulation.

### Phase 1: Collection (`src/ast/collector.rs`)

1.  **File Scanning:** The tool globs all files matching the `patterns` in your config.
2.  **Lexical Analysis:** It parses each file into an AST and extracts all `SingleThreadedComments`.
3.  **Directive Parsing:** When it encounters an `ExportDecl` (like `export const` or `export function`), it checks the leading comments for the `whitelabel:` prefix.
4.  **Data Extraction:** It extracts the physical file path, the exported symbol name, and maps it to the requested `target` and `key`.

### Phase 2: Generation (`src/generator/*.rs`)

1.  The tool groups the collected entries by their `target` (e.g., all entries for `brandA`).
2.  It writes `[target].generated.tsx` files, importing the original symbols from their physical paths and structuring them into a single object.
3.  It generates an `index.ts` file that imports all target files and exports a unified TypeScript union and config record.

### Phase 3: Codemod & Rewriting (`src/ast/rewriter.rs` & `src/ast/scanner.rs`)

This is the most complex phase. The tool must safely replace local variables with the global `whitelabel` object.

1.  **Path Resolution:** The `SymbolScanner` reads your `tsconfig.json` `paths` mapping. It intercepts `ImportDecl` nodes and resolves raw import strings (e.g., `@/components/Hero`) into absolute physical paths on the disk.
2.  **Tracking References:** It checks if the imported symbol matches a known whitelabel entry originating from that exact physical file. If it matches, it flags the AST identifier's internal `Id`.
3.  **AST Transformation:** The `WhitelabelRewriter` walks the AST and replaces the flagged identifiers:
    - **Standard Identifiers:** `foo` becomes `whitelabel.foo`.
    - **Object Shorthands:** `{ foo }` becomes `{ foo: whitelabel.foo }`.
    - **JSX Elements:** `<Foo />` becomes `<whitelabel.Foo />`.
4.  **Import Injection:** If a file was modified, the tool calculates a dynamic, safe relative path from the current file to the generated registry using `pathdiff`. It then injects `import whitelabel from "..."` at the top of the AST.
5.  **Rename Detection:** If you change a `key` in your magic comment, the tool parses the _old_ generated registry, detects the diff, and runs a secondary codemod to rename all `whitelabel.oldKey` to `whitelabel.newKey` across the entire codebase.

---

## ⚠️ Current Limitations

- **Named Re-exports:** Syntax like `export { foo as companyName }` is not supported. You must use direct inline exports.
- **Formatting:** The AST emitter outputs standard ES2022 code. It is highly recommended to pipe the modified files into Prettier or ESLint after running the tool.
