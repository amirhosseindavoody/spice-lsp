# LSP Features

LSP methods spice-lsp implements or plans to implement, organized by release phase. MVP ships only syntax sync and diagnostics; richer behavior accumulates in later versions.

## Capability matrix

| LSP method / feature | MVP | v0.2 | v0.3 | v0.4 | v0.5 |
|----------------------|-----|------|------|------|------|
| `initialize` / `initialized` | ✓ | | | | |
| `shutdown` / `exit` | ✓ | | | | |
| `textDocument/didOpen` / `didChange` / `didClose` | ✓ | | | | |
| `textDocument/publishDiagnostics` | ✓ | ✓ | | | syntax |
| Syntax diagnostics | ✓ | | | | |
| Duplicate / undefined symbol diagnostics | | ✓ | | | |
| Dangling node / floating net diagnostics | | | | | ✓ |
| `textDocument/documentSymbol` | | ✓ | | | |
| `textDocument/definition` / `references` | | ✓ | | | |
| `textDocument/completion` | | | ✓ | | |
| `textDocument/hover` (dialect reference) | | | ✓ | | curated `reference/` |
| `textDocument/hover` (file-local) | | | ✓ | | subcircuit pins, in-file models |
| `textDocument/formatting` | | | | ✓ | |
| `textDocument/didSave` (re-lint) | | | | ✓ | |

Full specification of reference hover and net diagnostics: [Dialect reference and net semantics](8_dialect-reference-and-semantics.md).

## MVP

### Server capabilities

```json
{
  "capabilities": {
    "textDocumentSync": {
      "openClose": true,
      "change": 2,
      "save": false
    }
  },
  "serverInfo": { "name": "spice-lsp", "version": "0.1.0" }
}
```

`change: 2` is **Incremental** sync — the client sends edit ranges, not the full buffer every keystroke.

Diagnostics arrive via server-initiated `textDocument/publishDiagnostics`.

### Syntax diagnostics

| Source | Example | Severity |
|--------|---------|----------|
| Unclosed `.subckt` | `missing .ends for subcircuit X` | Error |
| Parse ERROR node | `unexpected token` | Error |
| Missing CST child | `expected node list` | Error |

## v0.2 — Symbols and light semantics

### Server capabilities

Adds outline and navigation on top of MVP sync:

```json
{
  "capabilities": {
    "textDocumentSync": { "openClose": true, "change": 2 },
    "documentSymbolProvider": true,
    "definitionProvider": true,
    "referencesProvider": true
  }
}
```

**Document symbols** for outline / breadcrumbs:

| Symbol kind | SPICE construct |
|-------------|-----------------|
| `Namespace` | `.subckt` block |
| `Class` | `.model` |
| `Variable` | `.param` |
| `Field` | Instance line |

**Navigation:** go to definition and find references for subcircuits, models, and parameters. `textDocument/references` honors `context.includeDeclaration` (omit definition sites when the client passes `false`).

**Diagnostics:**

| Code | Example | Severity |
|------|---------|----------|
| `spice/duplicate-name` | `duplicate component name 'R1'` | Warning |
| `spice/unknown-model` | `model 'nfet' not defined` | Warning |

Diagnostics from `didChange` are **debounced** (~150 ms) so rapid typing does not re-analyze on every keystroke. `didOpen` publishes immediately. Navigation handlers refresh the in-memory index on demand.

## v0.3 — Completion and file-local hover

**Completion** contexts: element letters, directive names, in-scope model and subcircuit names, snippet templates for `.tran` / `.subckt`.

**Hover (file-local only):** subcircuit port order, parameters from `.model` / `.subckt` in the open buffer. No curated reference yet — that is v0.5.

## Dialect selection (issue #16)

`spiceLsp.dialect` is `hspice` \| `ngspice` \| `ltspice` (default **`hspice`**). The VS Code command **SPICE LSP: Set Dialect…** and a status-bar item change it. The same dialect selects the reference corpus for hover and (later) completion docs.

Design: [Multi-dialect support](internal/2_multi-dialect-design.md).

### Hover

`textDocument/hover` resolves in order:

1. Curated entry from `reference/` for the active dialect (`_shared` fallback)
2. File-local detail for `.subckt` / `.model` / `.param` symbols
3. No hover

## v0.4 — Formatting

Registers `documentFormattingProvider`. Formatter profiles may follow the active dialect later.

See [Formatter](6_formatter.md).

## v0.5 — Dialect reference hover and net connectivity

### Reference-powered hover

When the cursor is on a directive, option, element keyword, or documented expression form, the server loads the matching entry from `reference/<dialect>/` and returns markdown:

- Summary and syntax
- Parameter table with units
- Examples and `seeAlso` links

You maintain this corpus over time; the LSP only indexes and renders it. Authoring guide: [Dialect reference and net semantics](8_dialect-reference-and-semantics.md#part-1--dialect-reference-library).

### Connectivity diagnostics

| Code | Example | Severity |
|------|---------|----------|
| `spice/dangling-node` | `node 'bias' is connected to only one device terminal` | Warning |
| `spice/floating-net` | `net 'internal' has no DC path to ground` | Warning |

Published alongside syntax diagnostics in `publishDiagnostics`. Configurable via `spiceLsp.diagnostics.*` settings.

## Client configuration

| Setting | Type | Default | Available |
|---------|------|---------|-----------|
| `spiceLsp.dialect` | string | `"hspice"` | dialect switch (see [Multi-dialect design](internal/2_multi-dialect-design.md)) |
| `spiceLsp.diagnostics.danglingNodes` | boolean | `true` | v0.5+ |
| `spiceLsp.diagnostics.floatingNets` | boolean | `true` | v0.5+ |
| `spiceLsp.groundNodes` | string[] | `["0","gnd","GND"]` | v0.5+ |
| `spiceLsp.trace.server` | string | `"off"` | MVP+ |

## Testing

Each phase adds integration tests for newly advertised capabilities. MVP priorities:

1. `initialize` returns expected capabilities
2. Open invalid document → diagnostics notification
3. Edit → updated diagnostics

v0.2 adds:

4. `documentSymbol` returns hierarchical outline for `.subckt` blocks
5. `definition` on subcircuit reference jumps to `.subckt` definition
6. `references` on subcircuit definition lists definition + usages; `includeDeclaration: false` omits the definition
7. Semantic fixtures (`duplicate-instance.cir`, `unknown-subckt.cir`) produce warning codes
8. Rapid `didChange` events coalesce into a single diagnostics publish after the debounce window

9. (v0.5) Open semantic fixture → dangling / floating warnings; hover snapshot matches reference entry

See [Demo and testing](development/3_demo-and-test.md).
