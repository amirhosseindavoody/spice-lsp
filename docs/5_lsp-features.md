# LSP Features

Language Server Protocol methods spice-lsp implements or plans to implement. Status reflects the roadmap; MVP ships only the rows marked **MVP**.

## Capability matrix

| LSP method / feature | MVP | v0.2 | v0.3 | v0.4 | Notes |
|----------------------|-----|------|------|------|-------|
| `initialize` / `initialized` | ✓ | | | | Advertise sync + server info |
| `shutdown` / `exit` | ✓ | | | | Clean teardown |
| `textDocument/didOpen` | ✓ | | | | Full buffer sync |
| `textDocument/didChange` | ✓ | | | | Incremental content changes |
| `textDocument/didClose` | ✓ | | | | Drop document from store |
| `textDocument/didSave` | | | | ✓ | Optional re-lint on save |
| `textDocument/publishDiagnostics` | ✓ | | | | Syntax errors from parser |
| `textDocument/documentSymbol` | | ✓ | | | `.subckt`, `.model`, top-level instances |
| `textDocument/definition` | | ✓ | | | Jump to `.subckt` / `.model` / `.param` |
| `textDocument/references` | | ✓ | | | Find usages of symbols |
| `textDocument/hover` | | | ✓ | | Pin order, parameter docs |
| `textDocument/completion` | | | ✓ | | Elements, directives, `.model` names |
| `completionItem/resolve` | | | ✓ | | Extra detail for complex items |
| `textDocument/formatting` | | | | ✓ | Full-buffer format |
| `textDocument/rangeFormatting` | | | | ✓ | Selection format |
| `$/progress` | | | | ✓ | Long format on huge files |

## MVP server capabilities

The MVP `InitializeResult` should advertise at minimum:

```json
{
  "capabilities": {
    "textDocumentSync": {
      "openClose": true,
      "change": 2,
      "save": false
    }
  },
  "serverInfo": {
    "name": "spice-lsp",
    "version": "0.1.0"
  }
}
```

`change: 2` means **Incremental** sync — the client sends edit ranges rather than the full document on every keystroke.

Diagnostics are pushed via **`textDocument/publishDiagnostics`** (server → client notification), not a request/response pair.

## Diagnostics (MVP detail)

| Source | Example message | Severity |
|--------|-----------------|----------|
| Unclosed `.subckt` | `missing .ends for subcircuit X` | Error |
| Parse ERROR node | `unexpected token` | Error |
| Missing child node | `expected node list` | Error |
| (v0.2) Duplicate `R1` | `duplicate component name 'R1'` | Warning |
| (v0.2) Unknown model | `model 'nfet' not defined` | Warning |

Ranges must use LSP positions (0-based line/character, UTF-16 code units).

## Document symbols (v0.2)

Outline entries for the VS Code outline / breadcrumb UI:

| Symbol kind | SPICE construct |
|-------------|-----------------|
| `Namespace` | `.subckt` block |
| `Class` | `.model` definition |
| `Variable` | `.param` |
| `Field` | Top-level instance (R, C, L, …) |

## Navigation (v0.2)

- **Go to definition:** cursor on subcircuit call or model name → jump to `.subckt` / `.model`
- **Find references:** reverse index from definitions to usages

Requires a symbol table built during semantic analysis — see [Architecture](4_architecture.md).

## Completion (v0.3)

Context-aware triggers:

| Context | Suggestions |
|---------|-------------|
| Start of line | `R`, `C`, `L`, `V`, `I`, `M`, `X`, `.subckt`, `.model`, `.tran`, … |
| After element letter | Snippet with node placeholders |
| Model slot | Known `.model` names in scope |
| Subcircuit call | Known `.subckt` names |

Use LSP `CompletionItemKind` and `insertTextFormat: Snippet` for multi-stop templates.

## Formatting (v0.4)

See [Formatter](6_formatter.md). Registers as:

```json
"documentFormattingProvider": true,
"documentRangeFormattingProvider": true
```

## Client configuration (future)

Expose via VS Code `settings.json` and LSP `initializationOptions`:

| Setting | Type | Default | Purpose |
|---------|------|---------|---------|
| `spiceLsp.dialect` | `"ngspice" \| "ltspice" \| "hspice"` | `"ngspice"` | Grammar/quirk selection |
| `spiceLsp.trace.server` | `"off" \| "messages" \| "verbose"` | `"off"` | Debug LSP traffic |

## Testing LSP behavior

Integration tests should cover each implemented method. See [Demo and testing](development/3_demo-and-test.md).

Priority test cases for MVP:

1. `initialize` returns expected capabilities
2. Open document → receive diagnostics notification
3. Edit document → updated diagnostics
4. Close document → no crash; reopen works
