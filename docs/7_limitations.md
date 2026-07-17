# Limitations

Known constraints and unsupported behavior. Updated as the parser and LSP mature.

## Shipped today

- Rust crates (`spice-parser`, `spice-lsp`, `spice-reference`), Tree-sitter grammar, and VS Code extension
- Syntax diagnostics plus semantic warnings (`spice/duplicate-name`, `spice/unknown-model`, include/lib path issues)
- Document outline, go to definition, and find references
- Dialect-aware hover from the curated `reference/` corpus (default HSPICE) plus file-local pin/model detail
- `.include` / `.lib` resolution for model and subcircuit definitions
- Document formatting (`textDocument/formatting`) and `spice-lsp format` CLI
- Debounced diagnostics on edit; `textDocument/references` honors `includeDeclaration`
- Marketplace extension with bundled binaries, TextMate highlighting, and restart command
- File associations for `.cir`, `.sp`, `.spf`, `.net`, `.ckt`, `.inc`, and `.lib`

## Current limitations

| Limitation | Workaround |
|------------|------------|
| Shared grammar for all dialects | Prefer common SPICE constructs; dialect-specific parse quirks grow over time; hover/docs already switch |
| No connectivity analysis | Manual review until dangling/floating checks land |
| Include graph is definition-focused | `.include` / `.lib` resolve models and subcircuits for diagnostics and go-to-definition; outline and find-references stay file-local — see [Include and library resolution](9_include-and-lib-resolution.md) |
| No completion yet | Type element/directive names manually |
| Formatter has no dialect profiles yet | Shared alignment/casing rules for all dialects; see [Formatter](6_formatter.md) |
| Comment toggle uses `*` only | `;` and `$` are highlighted as comments; VS Code allows one `lineComment` |
| No Windows arm64 bundled binary | Set `spiceLsp.serverPath` or put `spice-lsp` on `PATH` |
| Linux bundled binary needs glibc 2.31+ | Upgrade the host OS, or build `spice-lsp` locally and set `spiceLsp.serverPath` |
| Bare numeric lines outside `.DATA` | Prefer `+` continuations or keep value rows inside `.DATA` … `.ENDDATA` |

## Dialect reference coverage

The reference library under `reference/` grows **incrementally**:

- Shared baseline covers common directives (`.subckt`, `.tran`, `.dc`, `.op`, `.ac`, …) and elements (`R`, `C`, `X`)
- **HSPICE** overlays expand analysis/control docs (`.data`, multi-mode `.dc`, `.op`, `.measure`, `.probe`, `.lib`, …) — see [Dialect reference catalog](reference/README.md)
- LTspice remains a stub corpus; missing entry → no hover (not an error)
- Reference describes **language** constructs, not simulator version release notes

See [Dialect reference and net semantics](8_dialect-reference-and-semantics.md).

## Connectivity analysis (planned)

Dangling-node and floating-net diagnostics will be **heuristic**:

| Limitation | Detail |
|------------|--------|
| Single file | Net connectivity still ignores `.include` until a full cross-file net graph exists |
| Ground aliases | Defaults to `0`, `gnd`, `GND`; exotic ground names may need config |
| False positives | Intentionally open probe points may warn until suppression exists |
| Not full ERC | Does not check layout, EM, or foundry rules |
| Ideal elements | Voltage sources and unusual topologies need careful graph rules |

These warnings supplement — not replace — simulator and layout review.

## Dialect differences

| Area | Ngspice | LTspice | HSPICE |
|------|---------|---------|--------|
| Comments | `*`, `;`, `$` | `$` common | `*` |
| Directives / options | Baseline corpus | Overrides in `reference/ltspice/` | Overrides in `reference/hspice/` |

Parsing is still largely Ngspice-oriented; reference namespaces already switch with `spiceLsp.dialect`.

## Parser robustness

- Error recovery may leave incomplete indexes until syntax is fixed
- Very large files still re-parse the full buffer after the debounce window
- LSP assumes UTF-8 source

## Editor / LSP

- UTF-16 positions per LSP spec
- Stdio transport only
- No workspace-wide symbol search yet (include graph is used for definitions, not `workspace/symbol`)
- Diagnostics on `didChange` are debounced (~150 ms); navigation requests re-analyze on demand so the index stays current

## Reporting issues

Include:

1. Minimal netlist snippet
2. Dialect (Ngspice / LTspice / HSPICE)
3. Expected vs actual diagnostic or hover text

Add a fixture under `test-data/` when fixing.
