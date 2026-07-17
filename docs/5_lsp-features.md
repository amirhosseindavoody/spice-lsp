# LSP Features

LSP methods spice-lsp implements today, plus a short note on planned work.

## Capability matrix

| LSP method / feature | Status |
|----------------------|--------|
| `initialize` / `initialized` | Shipped |
| `shutdown` / `exit` | Shipped |
| `textDocument/didOpen` / `didChange` / `didClose` | Shipped |
| `textDocument/publishDiagnostics` | Shipped |
| Syntax diagnostics | Shipped |
| Duplicate / undefined symbol diagnostics | Shipped |
| Include / `.lib` resolution diagnostics | Shipped |
| `textDocument/documentSymbol` | Shipped |
| `textDocument/definition` / `references` | Shipped |
| `textDocument/hover` (dialect reference + file-local) | Shipped |
| Dangling node / floating net diagnostics | Planned |
| `textDocument/formatting` / `rangeFormatting` | Shipped |
| `textDocument/completion` | Planned |
| `textDocument/didSave` (re-lint) | Planned |

Full specification of reference hover and net diagnostics: [Dialect reference and net semantics](8_dialect-reference-and-semantics.md).

## Server capabilities

```json
{
  "capabilities": {
    "textDocumentSync": {
      "openClose": true,
      "change": 2,
      "save": false
    },
    "documentSymbolProvider": true,
    "definitionProvider": true,
    "referencesProvider": true,
    "hoverProvider": true,
    "documentFormattingProvider": true,
    "documentRangeFormattingProvider": true
  },
  "serverInfo": { "name": "spice-lsp", "version": "0.1.0" }
}
```

`change: 2` is **Incremental** sync — the client sends edit ranges, not the full buffer every keystroke.

Diagnostics arrive via server-initiated `textDocument/publishDiagnostics`.

## Syntax diagnostics

| Source | Example | Severity |
|--------|---------|----------|
| Unclosed `.subckt` | `missing .ends for subcircuit X` | Error |
| Parse ERROR node | `unexpected token` | Error |
| Missing CST child | `expected node list` | Error |

## Symbols and navigation

**Document symbols** for outline / breadcrumbs:

| Symbol kind | SPICE construct |
|-------------|-----------------|
| `Namespace` | `.subckt` block |
| `Class` | `.model` |
| `Variable` | `.param` |
| `Field` | Instance line (full analysis only) |

**Navigation:** go to definition and find references for subcircuits, models, and parameters. `textDocument/references` honors `context.includeDeclaration` (omit definition sites when the client passes `false`).

**Include / library resolution:** `.include` / `.inc` and HSPICE `.lib 'file' entry` are followed so model and subcircuit definitions in those files participate in unknown-model checks and go-to-definition. On a `.lib 'file' entry` (or `.include` path) line, go to definition on the path opens that file; on the entry name it jumps to the matching `.lib entry` section header. See [Include and library resolution](9_include-and-lib-resolution.md).

### Large-file / extracted analysis

| Setting | Default | Effect |
|---------|---------|--------|
| `spiceLsp.analysisMode` | `auto` | `auto` / `full` / `extracted` |
| `spiceLsp.extractedByteThreshold` | `16777216` (16 MiB) | Size gate for `auto` |

In **extracted** mode (forced, or `auto` when the buffer reaches the threshold):

- Index keeps `.subckt` / `.model` / `.param` definitions
- Instance symbols and outline children are omitted
- `spice/duplicate-name` is not emitted
- Unknown-model still reports unique missing model/subckt names (sparse refs)
- Go to definition on an instance’s model/subckt token still works via line classification

Design detail: [Large-file / extracted mode](internal/3_large-file-extracted-mode.md).

## Semantic diagnostics

| Code | Example | Severity |
|------|---------|----------|
| `spice/duplicate-name` | `duplicate component name 'R1'` | Warning |
| `spice/unknown-model` | `model 'nfet' not defined` | Warning |
| `spice/include-not-found` | `include file not found: 'models.inc'` | Warning |
| `spice/lib-section-not-found` | `library section 'TT' not found` | Warning |
| `spice/include-cycle` | `include cycle involving '…'` | Warning |

Diagnostics from `didChange` are **debounced** (~150 ms) so rapid typing does not re-analyze on every keystroke. `didOpen` publishes immediately. Navigation handlers refresh the in-memory index on demand.

## Dialect selection

`spiceLsp.dialect` is `hspice` \| `ngspice` \| `ltspice` (default **`hspice`**). The VS Code command **SPICE LSP: Set Dialect…** and a status-bar item change it. The same dialect selects the reference corpus for hover and (later) completion docs.

Design: [Multi-dialect support](internal/2_multi-dialect-design.md).

## Hover

`textDocument/hover` resolves in order:

1. Curated entry from `reference/` for the active dialect (`_shared` fallback)
2. File-local detail for `.subckt` / `.model` / `.param` symbols
3. No hover

When the cursor is on a directive, option, element keyword, or documented expression form, the server loads the matching entry and returns markdown (summary, syntax, parameter table, examples). You maintain this corpus over time; the LSP indexes and renders it. Authoring guide: [Dialect reference and net semantics](8_dialect-reference-and-semantics.md#part-1--dialect-reference-library).

## Formatting

`textDocument/formatting` and `textDocument/rangeFormatting` return a full-document `TextEdit` when the buffer would change. Range formatting uses the same full-document pass so instance alignment groups stay consistent. LSP `tabSize` maps to continuation `indentWidth`; output always uses spaces.

Rules, CLI (`spice-lsp format`), and options: [Formatter](6_formatter.md).

## Planned

### Completion

Element letters, directive names, in-scope model and subcircuit names, snippet templates for `.tran` / `.subckt`. Documentation can attach the same reference entries used for hover.

### Connectivity diagnostics

| Code | Example | Severity |
|------|---------|----------|
| `spice/dangling-node` | `node 'bias' is connected to only one device terminal` | Warning |
| `spice/floating-net` | `net 'internal' has no DC path to ground` | Warning |

Published alongside other diagnostics in `publishDiagnostics`. Configurable via `spiceLsp.diagnostics.*` settings.

## Client configuration

| Setting | Type | Default | Notes |
|---------|------|---------|-------|
| `spiceLsp.dialect` | string | `"hspice"` | Dialect switch |
| `spiceLsp.libraryPaths` | string[] | `[]` | Include / `.lib` search path |
| `spiceLsp.include.maxDepth` | number | `16` | Nested include / `.lib` depth cap |
| `spiceLsp.diagnostics.danglingNodes` | boolean | `true` | Planned connectivity pass |
| `spiceLsp.diagnostics.floatingNets` | boolean | `true` | Planned connectivity pass |
| `spiceLsp.groundNodes` | string[] | `["0","gnd","GND"]` | Planned connectivity pass |
| `spiceLsp.trace.server` | string | `"off"` | LSP trace level |

## Testing

Integration coverage includes:

1. `initialize` returns expected capabilities
2. Open invalid document → diagnostics notification
3. Edit → updated diagnostics (debounced)
4. `documentSymbol` returns hierarchical outline for `.subckt` blocks
5. `definition` on subcircuit reference jumps to `.subckt` definition
6. `definition` on `.lib 'file' entry` path opens the library file; on the entry name jumps to `.lib entry`
7. `references` on subcircuit definition lists definition + usages; `includeDeclaration: false` omits the definition
7. Semantic fixtures (`duplicate-instance.cir`, `unknown-subckt.cir`) produce warning codes
8. Hover snapshots match reference entries for the active dialect

Planned: semantic fixtures for dangling / floating warnings once connectivity lands.

See [Demo and testing](development/2_demo-and-test.md).
