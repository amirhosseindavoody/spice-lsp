# LSP Features

LSP methods spice-lsp implements or plans to implement, organized by release phase. MVP ships only syntax sync and diagnostics; richer behavior accumulates in later versions.

## Capability matrix

| LSP method / feature | MVP | v0.2 | v0.3 | v0.4 | v0.5 |
|----------------------|-----|------|------|------|------|
| `initialize` / `initialized` | ✓ | | | | |
| `shutdown` / `exit` | ✓ | | | | |
| `textDocument/didOpen` / `didChange` / `didClose` | ✓ | | | | |
| `textDocument/publishDiagnostics` | ✓ | | | | syntax |
| Syntax diagnostics | ✓ | | | | |
| Duplicate / undefined symbol diagnostics | | ✓ | | | |
| Dangling node / floating net diagnostics | | | | | ✓ |
| `textDocument/documentSymbol` | | ✓ | | | |
| `textDocument/definition` / `references` | | ✓ | | | |
| `textDocument/completion` | | | ✓ | | |
| `textDocument/hover` (file-local) | | | ✓ | | subcircuit pins, in-file models |
| `textDocument/hover` (dialect reference) | | | | | ✓ |
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

**Document symbols** for outline / breadcrumbs:

| Symbol kind | SPICE construct |
|-------------|-----------------|
| `Namespace` | `.subckt` block |
| `Class` | `.model` |
| `Variable` | `.param` |
| `Field` | Instance line |

**Navigation:** go to definition and find references for subcircuits, models, and parameters.

**Diagnostics:**

| Code | Example | Severity |
|------|---------|----------|
| `spice/duplicate-name` | `duplicate component name 'R1'` | Warning |
| `spice/unknown-model` | `model 'nfet' not defined` | Warning |

## v0.3 — Completion and file-local hover

**Completion** contexts: element letters, directive names, in-scope model and subcircuit names, snippet templates for `.tran` / `.subckt`.

**Hover (file-local only):** subcircuit port order, parameters from `.model` / `.subckt` in the open buffer. No curated reference yet — that is v0.5.

## v0.4 — Formatting and dialect

Registers `documentFormattingProvider`. Dialect setting (`ngspice` | `ltspice` | `hspice`) selects grammar and reference namespace.

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
| `spiceLsp.dialect` | string | `"ngspice"` | v0.4+ |
| `spiceLsp.diagnostics.danglingNodes` | boolean | `true` | v0.5+ |
| `spiceLsp.diagnostics.floatingNets` | boolean | `true` | v0.5+ |
| `spiceLsp.groundNodes` | string[] | `["0","gnd","GND"]` | v0.5+ |
| `spiceLsp.trace.server` | string | `"off"` | MVP+ |

## Testing

Each phase adds integration tests for newly advertised capabilities. MVP priorities:

1. `initialize` returns expected capabilities
2. Open invalid document → diagnostics notification
3. Edit → updated diagnostics
4. (v0.5) Open semantic fixture → dangling / floating warnings; hover snapshot matches reference entry

See [Demo and testing](development/3_demo-and-test.md).
