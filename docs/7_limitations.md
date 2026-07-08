# Limitations

Known constraints, unsupported constructs, and honest gaps. Update this page as the parser and LSP mature.

## Current (pre-MVP scaffolding)

- No Rust crates or language server binary yet
- No Tree-sitter grammar in the repository
- No VS Code extension package
- Documentation describes intended behavior

## MVP limitations

When MVP ships, expect:

| Limitation | Workaround |
|------------|------------|
| **Ngspice dialect only** | Avoid LTspice-specific directives until dialect support lands |
| **Syntax diagnostics only** | Simulator still catches many semantic errors at run time |
| **No formatting** | Manual alignment; formatter comes in v0.4 |
| **No completion or navigation** | Use text search for subcircuit/model names |
| **Single-file analysis** | `.include` / `.lib` not followed across files |
| **No preprocessor** | `.if` / `.ifdef` blocks parsed as generic text |

## Dialect differences (planned handling)

SPICE variants diverge on directives, units, and device syntax:

| Area | Ngspice | LTspice | HSPICE |
|------|---------|---------|--------|
| Comment leaders | `*`, `;`, `$` | `$` common | `*` |
| Instance prefix | Standard | `NPN`, `PNP`, behavioral extras | Vendor extensions |
| `.model` syntax | Common subset | LTspice-specific params | HSPICE levels |

Full tri-dialect support is a **v0.4+** goal. MVP picks one dialect to avoid false positives.

## Parser robustness

Tree-sitter error recovery means:

- Partial trees after errors — diagnostics may be incomplete until the syntax is fixed
- Very large files (> 100k lines) may need debouncing and profiling
- Malformed UTF-8 is rejected at the LSP layer (LSP assumes UTF-8 documents)

## Editor / LSP limitations

- **UTF-16 positions:** Clients must match LSP's UTF-16 code unit indexing for non-ASCII identifiers (rare in SPICE)
- **Stdio only (MVP):** TCP/socket transport not planned initially
- **No workspace symbols:** Cross-file index requires include-graph analysis (future)

## Simulator vs LSP

The LSP does **not** guarantee netlist validity for simulation:

- Floating nodes, missing grounds, and convergence issues are simulator concerns
- Light semantic checks (duplicate names, missing models) come in v0.2 but will never replace a full ERC/DRC tool

## Reporting gaps

If you hit a construct that parses incorrectly, open an issue with:

1. Minimal netlist snippet
2. Target dialect (Ngspice / LTspice / HSPICE)
3. Expected vs actual diagnostic (or missing diagnostic)

Add a fixture under `test-data/` when fixing.
