# 🏷️ Whitelabel Extractor (wl-extractor)

> **⚠️ Status: In Active Development** > This project is currently in its early stages and is undergoing rapid iteration to ensure library-grade robustness. While the core codemod architecture is mathematically sound and tested, please be aware that breaking changes may occur. To see our immediate priorities, known limitations, and upcoming milestones, please review the [Roadmap in `CONTRIBUTION.md`](./CONTRIBUTION.md).

Welcome to the **Whitelabel Extractor**! This is a lightning-fast, Rust-based AST transformation tool built on top of [SWC](https://swc.rs/).

In multi-tenant applications, codebases often become cluttered with conditional logic (`if (brand === 'X')`) to render different configurations, strings, or UI components for different targets. `wl-extractor` solves this by allowing developers to write standard, localized variables. The tool automatically extracts those variables, generates a strictly typed registry, and safely rewrites your codebase to use them dynamically.

## 🚀 Features

It runs an SWC-powered AST (Abstract Syntax Tree) traversal over your codebase to do three things:

1. **Extracts:** Finds any variable or function exported with a `// whitelabel` comment.
2. **Generates:** Builds a central TypeScript registry (`brandA.generated.tsx`, `index.ts`, etc.) based on those exports.
3. **Rewrites (Codemod):** Goes through your files (like Next.js `page.tsx` files) and surgically replaces direct imports of those components/variables with property accesses on the generated `whitelabel` object.

It resolves paths using Node.js rules and respects your `tsconfig.json` path aliases (e.g., `@/components/*`).

---

## ⚙️ Configuration

Before running the tool, create a `whitelabel.config.json` file in the root of your project:

```json
{
  "src": "src/",
  "patterns": ["**/*.tsx", "**/*.ts"],
  "output_dir": "whitelabel",
  "default_target": "def",
  "tsconfig": "tsconfig.json"
}
```

---

## 🛠️ Authoring Guide

### 1. Mark your Exports

To mark a variable, function, or component for extraction, simply add a magic comment directly above its `export` statement. 

The directive parser uses a formal Context-Free Grammar (CFG), allowing for a highly flexible, natural-language syntax:

```text
WHITELABEL [ ":" ] [ modifier [, modifier ...] ]

modifier:
    *
  | FOR [ "=" | ":" ] value
  | KEY ( "=" | ":" ) value
  | AS  [ "=" | ":" ] value
  | OPTIONAL

value:
    string
  | "'" string "'"
  | '"' string '"'
```

**Available Modifiers (Case-Insensitive):**

* **`for`**: (Optional) The specific brand/tenant this code applies to. You can use `=`, `:`, or omit the operator entirely (e.g., `for=variant1`, `for:'variant1'`, or `for variant1`). If omitted, it falls back to the `default_target` defined in your config.
* **`key`**: (Optional) A custom key for the registry. Requires `=` or `:` (e.g., `key=BG_COLOR`). If omitted, the tool uses the variable's original exported name.
* **`as`**: (Optional) An ergonomic alias for `key` that does not require an operator (e.g., `as BG_COLOR`).
* **`optional`**: (Optional) Flags the extraction as optional for the target.
* **`*`**: (Optional) A wildcard modifier indicating the extraction applies to all targets.

*Note: The `:` immediately following `whitelabel` is entirely optional. Modifier values can be unquoted, single-quoted (`'...'`), or double-quoted (`"..."`).*

**Examples:** [`basic-usages` fixture input](tests/fixtures/integrations/basic-usages/app/home/page.tsx)
```ts
/**
 * 1. The most formal `whitelabel` marker
 */
// whitelabel: for=variant1, key=BG_COLOR
export const variant1_bgClassname: string = "bg-red-100";

/**
 * 2. If `for` is omitted, it defaults to your config's `default_target`
 */
// whitelabel
export const BG_COLOR: string = "bg-red-200";

/**
 * 3. The most natural `whitelabel` marker. 
 * `as` works identically to `key`, but allows you to omit the operator.
 */
// whitelabel for 'variant2' as 'BG_COLOR'
export const variant2_bgClassname: string = "bg-red-300";

/**
 * 4. Operators and quotes are entirely optional.
 */
// whitelabel for variant3 as BG_COLOR
export const variant3_bgClassname: string = "bg-red-400";
```

### 2. Run the Extractor

Execute the CLI tool in your terminal:

```bash
cargo install --path . ; wl-extractor
```

_(Tip: For CI/CD or piping to Prettier, use `wl-extractor --file-name-only` to suppress human-readable logs.)_

### 3. See the Magic (The Result)

The tool will automatically generate a registry in your `output_dir` containing:

1.  `brandA.generated.tsx` and `brandB.generated.tsx`
2.  An `index.ts` that unites them into a strictly typed `WhitelabelConfig`.
3.  A `determine-whitelabel.ts` file that uses `process.env.NEXT_PUBLIC_WHITELABEL` to select the current brand.

**Most importantly, it rewrites your local usages.** If you imported `BrandAHeader` somewhere else in your code, the tool transforms it:

**Before:** [basic-usages/app/home/page.tsx](./tests/fixtures/integrations/basic-usages/app/home/page.tsx)

```tsx
import { Heading } from "./_components/heading";

// whitelabel: key=BG_COLOR
export const bgClassname = "bg-red-500";

// whitelabel: for=variant1, key=BG_COLOR
export const variant1_bgClassname = "bg-green-500";

const Homepage = () => (
  <div className={`h-full w-full ${bgClassname}`}>
    <Heading />
  </div>
);

export default Homepage;
```

**After:** [test snapshot](./tests/snapshots/integration_test__tests__basic-usages.snap#L10-L24)

```tsx
import whitelabel from "../whitelabel"; // Injected automatically!
import { Heading } from "./_components/heading";

// whitelabel: key=BG_COLOR
export const bgClassname = "bg-red-500";

// whitelabel: for=variant1, key=BG_COLOR
export const variant1_bgClassname = "bg-green-500";

const Homepage = () => (
  // Surgically rewritten to use the global registry
  <div className={`h-full w-full ${whitelabel.BG_COLOR}`}>
    <whitelabel.Heading />
  </div>
);

export default Homepage;
```

---

## 🛑 Current Limitations (What it rejects)

This tool works like magic, but even magic needs a strict set of rules.

When dealing with automated AST (Abstract Syntax Tree) transformations, trying to support every single edge case of the TypeScript specification is a one-way ticket to brittle, unpredictable builds. **v1 explicitly ignores pure types and complex declarations**.

If you attempt to attach a `// whitelabel` directive to the following syntax, the CLI will throw a validation error and skip the extraction safely:

- ❌ **Types & Interfaces:** (`export type Config = {}`, `export interface Props {}`)
- ❌ **Enums:** (`export enum Colors {}`)
- ❌ **Classes:** (`export class ApiClient {}`)
- ❌ **Namespaces/Modules:** (`export module Utils {}`)
- ❌ **Named Re-exports:** (`export { foo as companyName }`)

_(💡 **Note:** You can still use classes, enums, and complex types extensively throughout your codebase! You just cannot tag them as the root whitelabel target to be extracted.)_

---

## 🧠 How It Works (Under the Hood)

The extractor operates in three distinct phases using `swc_core` for robust Abstract Syntax Tree (AST) manipulation.

### Phase 1: Collection (`src/ast/collector.rs`)

1.  **File Scanning:** The tool globs all files matching the `patterns` in your config.
2.  **Lexical Analysis:** It parses each file into an AST and extracts all `SingleThreadedComments`.
3. **LALRPOP Parsing:** When it encounters an `ExportDecl` (like `export const` or `export function`), it uses a mathematically sound Context-Free Grammar (powered by LALRPOP) to safely parse leading comments for directives.
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
