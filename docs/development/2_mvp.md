# MVP Guide

Step-by-step path to a **demoable minimum viable product**: a VS Code window that shows live syntax diagnostics on SPICE netlists.

Time estimate is intentionally omitted — each milestone is a mergeable unit you can verify before continuing.

## What MVP looks like

When done, you can:

1. Press **F5** in VS Code to open the Extension Development Host
2. Open a `.cir` file with a deliberate syntax error
3. See a diagnostic squiggle within one edit cycle
4. Fix the error and watch the squiggle disappear
5. Run `pixi run test` and have it pass in CI

Nothing else is required for MVP.

## Milestone checklist

```
[ ] M1  Cargo workspace + empty crates
[ ] M2  Tree-sitter grammar (minimal Ngspice subset)
[ ] M3  Parser crate: parse → syntax diagnostics
[ ] M4  LSP crate: stdio server, sync, publishDiagnostics
[ ] M5  test-data fixtures + automated tests
[ ] M6  VS Code extension spawns server
[ ] M7  End-to-end demo script documented
```

---

## M1 — Cargo workspace

Create the Rust workspace skeleton.

**Files:**

```
Cargo.toml                    # workspace root
crates/spice-parser/Cargo.toml
crates/spice-parser/src/lib.rs
crates/spice-lsp/Cargo.toml
crates/spice-lsp/src/main.rs
```

**Root `Cargo.toml`:**

```toml
[workspace]
members = ["crates/spice-parser", "crates/spice-lsp"]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
```

**`crates/spice-lsp/Cargo.toml` dependencies:**

```toml
spice-parser = { path = "../spice-parser" }
tower-lsp = "0.20"
tokio = { version = "1", features = ["full"] }
```

Add pixi tasks: `build`, `test`, `spice-lsp` — see [Build](1_build.md).

**Verify:** `pixi run cargo build` succeeds (empty server that exits immediately is fine).

---

## M2 — Tree-sitter grammar (minimal)

Start small. The grammar must recognize:

- Comments (`*`, `;`, `$` line comments)
- Instance lines: letter + name + nodes + value/model
- Dot directives: `.subckt`, `.ends`, `.model`, `.tran`, `.end`
- Continuation lines starting with `+`
- Bare numeric / engineering-value rows (HSPICE `.DATA` tables without leading `+`)

**Layout:**

```
tree-sitter-spice/
├── grammar.js
├── src/           # generated parser (or via tree-sitter CLI)
├── bindings/rust/
└── queries/
    └── highlights.scm   # optional for MVP
```

**Bootstrap option:** If grammar authoring is blocking, use a **line classifier** for MVP only (regex per line type) and replace with Tree-sitter in M2.1 — but prefer Tree-sitter from the start so incremental parse is free later.

**Verify:** Parse `test-data/valid/simple-rc.cir` without ERROR nodes; parse `test-data/invalid/unclosed-subckt.cir` with expected ERROR/MISSING nodes.

---

## M3 — Parser crate

`crates/spice-parser/src/lib.rs` exposes:

```rust
pub struct ParseResult {
    pub diagnostics: Vec<Diagnostic>,  // internal type, map in LSP crate
}

pub fn analyze(source: &str) -> ParseResult;
```

Implementation:

1. Run Tree-sitter parse
2. Walk tree for ERROR / MISSING nodes
3. Add hand-written checks: `.subckt` without matching `.ends` (simple stack)

Map byte offsets to line/column for LSP in the LSP crate (UTF-16 conversion helper).

**Verify:** `pixi run cargo test -p spice-parser` with fixtures in `test-data/`.

---

## M4 — LSP server

`crates/spice-lsp/src/main.rs` using tower-lsp:

```rust
#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend { client, docs: ... });
    Server::new(stdin, stdout, socket).serve(service).await;
}
```

**Backend implements:**

| Event | Action |
|-------|--------|
| `initialize` | Return capabilities (incremental sync) |
| `did_open` | Store document, analyze, `publish_diagnostics` |
| `did_change` | Apply edits, re-analyze, publish |
| `did_close` | Remove document |

Keep a `HashMap<Url, String>` (or versioned document struct) in the backend.

**Verify:** Integration test sends initialize + didOpen over async stdin/stdout; assert diagnostics notification JSON. See [Demo and testing](3_demo-and-test.md).

---

## M5 — Fixtures and tests

**Directory structure:**

```
test-data/
├── valid/
│   ├── simple-rc.cir
│   └── subckt.cir
└── invalid/
    ├── unclosed-subckt.cir
    └── bad-continuation.cir
```

**Parser tests:** For each invalid file, assert diagnostic count ≥ 1 and message substring.

**LSP tests:** Use `tower-lsp` test helpers or raw JSON-RPC frames:

1. Initialize
2. Open invalid file
3. Read notification until `publishDiagnostics`
4. Assert URI and range match

**Verify:** `pixi run test` green locally.

---

## M6 — VS Code extension

Scaffold under `editors/vscode/`:

```
editors/vscode/
├── package.json
├── tsconfig.json
├── src/extension.ts
└── language-configuration.json
```

**`package.json` essentials:**

```json
{
  "name": "spice-lsp",
  "displayName": "SPICE Language Support",
  "activationEvents": ["onLanguage:spice"],
  "contributes": {
    "languages": [{
      "id": "spice",
      "extensions": [".cir", ".sp", ".spf", ".net", ".ckt"]
    }],
    "configuration": {
      "properties": {
        "spiceLsp.serverPath": {
          "type": "string",
          "default": "",
          "description": "Path to spice-lsp binary (empty = bundled or PATH)"
        }
      }
    }
  }
}
```

**`extension.ts`:** Use `vscode-languageclient/node` to spawn the Rust binary:

```typescript
const serverOptions: ServerOptions = {
  command: config.get<string>("serverPath") || "spice-lsp",
  args: [],
};
const client = new LanguageClient("spiceLsp", "SPICE LSP", serverOptions, clientOptions);
await client.start();
```

For development, set `spiceLsp.serverPath` to `../../target/debug/spice-lsp` (absolute path in launch config).

**Verify:** F5 → open invalid netlist → squiggle appears.

Full detail: [VS Code integration](4_vscode-integration.md).

---

## M7 — Demo script

Document a 2-minute demo for reviewers:

1. `pixi run build`
2. Open VS Code on `editors/vscode`, press F5
3. In Extension Development Host: **File → Open** → `test-data/invalid/unclosed-subckt.cir`
4. Point out diagnostic message
5. Add missing `.ends`, save — diagnostic clears
6. Run `pixi run test` in terminal to show automation

Optional: record asciinema or short screen capture for README.

---

## What to skip until after MVP

Do **not** implement these before M7 is done:

- Formatter
- Completion, hover, go-to-definition, references
- Multi-dialect switching and `reference/` corpus
- `.include` file resolution *(now shipped — [Include and library resolution](../9_include-and-lib-resolution.md))*
- Semantic analysis (duplicate names, undefined models, dangling nodes, floating nets)
- Published VSIX / Marketplace listing (side-load is enough for MVP demo)

After MVP, follow the phase order in [Architecture](../4_architecture.md). Dialect reference hover and net connectivity are **v0.5** — see [Dialect reference and net semantics](../8_dialect-reference-and-semantics.md).

---

## Suggested merge strategy

| PR | Contents |
|----|----------|
| PR 1 | M1 + M2 + parser fixtures |
| PR 2 | M3 + M5 parser tests |
| PR 3 | M4 + LSP integration tests |
| PR 4 | M6 + M7 docs update |

Each PR should keep `pixi run test` green (or skip tests until the crate exists, then enable).

---

## Related

- [Principles](../3_principles.md) — MVP scope boundary
- [Architecture](../4_architecture.md) — where code lives
- [Demo and testing](3_demo-and-test.md)
- [VS Code integration](4_vscode-integration.md)
- [Dialect reference and net semantics](../8_dialect-reference-and-semantics.md) — post-MVP direction
