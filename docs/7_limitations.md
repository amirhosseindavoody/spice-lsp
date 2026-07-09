# Limitations

Known constraints and unsupported behavior. Updated as the parser and LSP mature.

## Current (v0.2)

Shipped today:

- Rust crates (`spice-parser`, `spice-lsp`), Tree-sitter grammar, and VS Code extension
- Syntax diagnostics plus light semantic warnings (`spice/duplicate-name`, `spice/unknown-model`)
- Document outline, go to definition, and find references
- Debounced diagnostics on edit; `textDocument/references` honors `includeDeclaration`
- Marketplace extension with bundled binaries, TextMate highlighting, and restart command
- File associations for `.cir`, `.sp`, `.spf`, `.net`, and `.ckt`

## Current limitations

| Limitation | Workaround |
|------------|------------|
| Ngspice-oriented parsing | Avoid LTspice/HSPICE-specific syntax until v0.4 |
| No connectivity analysis | Manual review; dangling/floating checks arrive in v0.5 |
| Single-file analysis | `.include` not followed |
| No formatter or completion | Manual alignment / typing until v0.3–v0.4 |
| Comment toggle uses `*` only | `;` and `$` are highlighted as comments; VS Code allows one `lineComment` |
| No Windows arm64 bundled binary | Set `spiceLsp.serverPath` or put `spice-lsp` on `PATH` |
| Linux bundled binary needs glibc 2.31+ | Upgrade the host OS, or build `spice-lsp` locally and set `spiceLsp.serverPath` |
| Shared grammar for all dialects | Dialect-specific parse quirks land in later phases; hover/docs already switch |

## Post-MVP: dialect reference

The reference library under `reference/` will grow **incrementally**:

- Early v0.5 may document only common directives and elements for Ngspice
- LTspice and HSPICE entries added over time; missing entry → no hover (not an error)
- Reference describes **language** constructs, not simulator version release notes

See [Dialect reference and net semantics](8_dialect-reference-and-semantics.md).

## Post-MVP: connectivity analysis

Dangling-node and floating-net diagnostics are **heuristic**:

| Limitation | Detail |
|------------|--------|
| Single file | Ignores nets defined across `.include` until cross-file index exists |
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

Tri-dialect parsing and reference namespaces target **v0.4–v0.5**. Current parsing is Ngspice-oriented.

## Parser robustness

- Error recovery may leave incomplete indexes until syntax is fixed
- Very large files still re-parse the full buffer after the debounce window
- LSP assumes UTF-8 source

## Editor / LSP

- UTF-16 positions per LSP spec
- Stdio transport only
- No workspace-wide symbol search until include-graph analysis exists
- Diagnostics on `didChange` are debounced (~150 ms); navigation requests re-analyze on demand so the index stays current

## Reporting issues

Include:

1. Minimal netlist snippet
2. Dialect (Ngspice / LTspice / HSPICE)
3. Expected vs actual diagnostic or hover text

Add a fixture under `test-data/` when fixing.
