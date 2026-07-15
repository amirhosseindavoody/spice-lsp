# Include and library resolution

How spice-lsp resolves `.model`, `.subckt`, and related symbols across `.include` / `.inc` and HSPICE `.lib` files.

## Goals

| Capability | Behavior |
|------------|----------|
| Follow `.include` / `.inc` | Load the target file and merge its model/subcircuit definitions into resolution |
| Follow `.lib 'file' entry` | Load only the named `.LIB entry` … `.ENDL` section from the library file |
| Unknown model/subckt | `spice/unknown-model` is suppressed when the name is defined in a reachable include or lib section |
| Go to definition | Jumps to the defining `.model` / `.subckt` in the included or library file |
| Missing path | `spice/include-not-found` (or `spice/lib-section-not-found`) on the include/lib directive |

Outline (`documentSymbol`) stays **file-local**. Find references stays in the open buffer for this phase (cross-file references may expand later).

## Directive shapes

| Form | Meaning |
|------|---------|
| `.include path` / `.inc path` | Insert the whole file |
| `.lib 'path' entry` | Call the named section in a library file (HSPICE-style) |
| `.lib entry` … `.endl` | Section delimiters **inside** a library file (not a file call) |

Paths may be single-quoted, double-quoted, or bare. Relative paths resolve against the including file’s directory, then against `spiceLsp.libraryPaths`.

## Resolution algorithm

```
analyze(root)
  → parse + local Index + IncludeRef list
  → for each IncludeRef (depth-limited, cycle-safe):
        resolve path (relative → libraryPaths → fail)
        load text (open buffer if present, else disk)
        if LibCall: keep only lines inside matching .LIB entry … .ENDL
        else: use full file
        build Index for that slice
        recurse into nested includes
  → merge external definitions
  → drop spice/unknown-model when name exists in merge
  → emit spice/include-not-found / spice/lib-section-not-found
```

Default max nesting depth is **16** (aligned with common HSPICE nested-`.LIB` limits).

## Search path

1. Absolute path as written
2. Relative to the directory of the file that contains the `.include` / `.lib` call
3. Each entry in `spiceLsp.libraryPaths` (workspace or absolute folders)

## LSP integration

| Request | Cross-file behavior |
|---------|---------------------|
| `publishDiagnostics` | Uses include graph when publishing for an open document |
| `textDocument/definition` | May return a `Location` in another file URI |
| `textDocument/references` | Same-buffer only (this phase) |
| `textDocument/documentSymbol` | Same-buffer only |
| `textDocument/hover` | File-local symbols still prefer the open buffer; dialect corpus unchanged |

Open buffers win over disk content when the resolved path matches an open document (so edits to an included file are visible before save).

## Configuration

| Setting | Type | Default | Purpose |
|---------|------|---------|---------|
| `spiceLsp.libraryPaths` | `string[]` | `[]` | Extra directories for resolving include/lib paths |
| `spiceLsp.include.maxDepth` | `number` | `16` | Cap on nested include/lib depth |

## Diagnostics

| Code | When |
|------|------|
| `spice/include-not-found` | Path does not resolve under search rules |
| `spice/lib-section-not-found` | File loads but the requested `.LIB` entry is missing |
| `spice/include-cycle` | Include/lib graph would revisit a file already on the stack |
| `spice/unknown-model` | Model/subckt still missing after the include closure is merged |

## Limits (this phase)

- No workspace-wide `workspace/symbol` yet
- No automatic PDK discovery beyond `libraryPaths`
- LTspice / Ngspice `.lib` quirks beyond the HSPICE call/section pattern are best-effort
- Nested `.lib` section selection inside an already-filtered section follows nested includes normally
