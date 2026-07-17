# Architecture

System layout for spice-lsp: crates, data flow, and how analysis layers build on each other.

## Story in four layers

Every feature belongs to one of these layers:

| Layer | Responsibility | Status |
|-------|----------------|--------|
| **1. Parse** | Tree-sitter CST, syntax diagnostics | Shipped |
| **2. Index** | Symbols, scopes, cross-references, include/lib graph | Shipped |
| **3. Assist** | Hover (reference + file-local); completion | Hover shipped; completion planned |
| **4. Deep semantics** | Formatter; net connectivity | Formatter shipped; connectivity planned |

Layer 4 and the reference corpus are documented in [Dialect reference and net semantics](8_dialect-reference-and-semantics.md).

## High-level overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Editor clients                           │
│   VS Code extension  │  Neovim  │  Helix  │  other LSP clients  │
└────────────┬────────────────────────────────────────────────────┘
             │  JSON-RPC 2.0 over stdio (LSP)
             ▼
┌─────────────────────────────────────────────────────────────────┐
│  crates/spice-lsp          (binary: spice-lsp)                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ tower-lsp Backend                                        │   │
│  │  • text sync, publishDiagnostics                         │   │
│  │  • symbols, definition, references                       │   │
│  │  • hover (reference corpus + file-local)                 │   │
│  │  • formatting (`format_source` → TextEdit)               │   │
│  │  • (planned) completion                                  │   │
│  └────────────────────────┬─────────────────────────────────┘   │
└───────────────────────────┼─────────────────────────────────────┘
                            │
          ┌─────────────────┼─────────────────┐
          ▼                 ▼                 ▼
┌─────────────────┐ ┌────────────────┐ ┌──────────────────┐
│ spice-parser    │ │ spice-reference│ │ tree-sitter-spice│
│ parse, index,   │ │ dialect docs   │ │ grammar, queries │
│ diagnose, format│ │                │ │                  │
└─────────────────┘ └────────────────┘ └──────────────────┘
```

## Crate responsibilities

| Crate / directory | Role |
|-------------------|------|
| `crates/spice-lsp` | LSP server, JSON-RPC, document store; `format` CLI subcommand |
| `crates/spice-parser` | Parsing, symbol index, diagnostics, formatter (`format_source`) |
| `crates/spice-reference` | Load and query dialect reference entries |
| `tree-sitter-spice/` | Grammar and query files |
| `reference/` | Curated JSON per dialect — authored over time |
| `editors/vscode/` | VS Code extension client |
| `test-data/` | Fixtures for syntax, semantics, hover snapshots |

## LSP server lifecycle

1. **Client connects** via stdio; sends `initialize` with client capabilities and dialect option.
2. **Server responds** with capabilities (incremental sync, diagnostics, symbols, definition, references, hover, formatting).
3. **Document open/change** updates an in-memory map of open buffers.
4. **On each change** (debounced ~150 ms):
   - Re-parse with Tree-sitter
   - Run diagnostic passes (syntax + semantic + include resolution)
   - Send `textDocument/publishDiagnostics` with the document version
5. **Hover** resolves against the CST and `spice-reference`. Navigation requests re-analyze on demand so the symbol index stays current even when diagnostics are still debouncing.
6. **Formatting** pretty-prints the buffer via `spice_parser::format_source` and returns a full-document `TextEdit` when needed.
7. **Shutdown** exits cleanly.

### Document model

```rust
struct Document {
    uri: Url,
    text: String,
    tree: tree_sitter::Tree,
    version: i32,
    symbols: SymbolTable,
    // planned
    net_graph: Option<NetGraph>,
}
```

## Parser and analysis pipeline

### Syntax

1. Parse buffer → CST
2. Collect ERROR / MISSING nodes and hand-written checks (e.g. unclosed `.subckt`)
3. Map to LSP `Diagnostic` (Error)

### Symbol index

Walk the CST to build:

- Subcircuit and model definitions
- Component instances and `.param` bindings

Enables navigation, duplicate-name warnings, and undefined reference checks.

### Include / library graph

Follow `.include` / `.inc` and HSPICE `.lib 'file' entry` (section-filtered) to merge external model and subcircuit definitions. Used by unknown-model diagnostics and go-to-definition. Details: [Include and library resolution](9_include-and-lib-resolution.md).

### Assist

Use the symbol index and reference corpus for **hover** (subcircuit pin lists, in-file `.model` parameters, curated directive/element docs). Completion will reuse the same index and corpus.

### Format and dialect

Dialect setting selects reference namespace and (later) grammar quirks / formatter profiles. The formatter pretty-prints line tokens (column alignment, `+` wrap, directive casing) and returns a full-document `TextEdit` — see [Formatter](6_formatter.md).

### Reference docs and connectivity

**Reference lookup (shipped):** Map cursor token → `reference/<dialect>/…` entry → markdown hover.

**Net graph (planned):** Build terminal graph per scope → warn on dangling nodes and floating nets.

```
Instance lines ──► NetGraph ──► dangling / floating diagnostics
Cursor token   ──► ReferenceIndex ──► rich hover markdown
```

See [Dialect reference and net semantics](8_dialect-reference-and-semantics.md).

## VS Code extension

Thin Node client: spawns `spice-lsp`, forwards LSP traffic, exposes dialect and diagnostic settings. No parsing in TypeScript.

See [VS Code integration](development/3_vscode-integration.md).

## Performance targets

| Metric | Target |
|--------|--------|
| Parse + syntax diagnose (5k lines) | < 50 ms |
| Full semantic pass + net graph (50k lines) | < 100 ms |
| Reference hover lookup | < 1 ms (in-memory index) |
| Incremental edit | Re-parse changed regions only |

## Related reading

- [Dialect reference and net semantics](8_dialect-reference-and-semantics.md)
- [LSP features](5_lsp-features.md) — method-by-method status
- [Demo and testing](development/2_demo-and-test.md) — verification
- [Design (internal)](internal/1_design.md) — full requirements
