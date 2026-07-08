# Limitations

Known constraints and unsupported behavior. Updated as the parser and LSP mature.

## Current (pre-MVP scaffolding)

- No Rust crates, grammar, or VS Code extension yet
- Documentation describes intended behavior and phased delivery

## MVP limitations

| Limitation | Workaround |
|------------|------------|
| Ngspice dialect only | Avoid LTspice/HSPICE-specific syntax until v0.4 |
| Syntax diagnostics only | Simulator catches many semantic errors at run time |
| No hover or reference docs | Consult simulator manual; reference corpus arrives in v0.5 |
| No connectivity analysis | Manual review; dangling/floating checks arrive in v0.5 |
| Single-file analysis | `.include` not followed |
| No formatter | Manual alignment until v0.4 |

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

Tri-dialect parsing and reference namespaces target **v0.4–v0.5**. MVP uses one dialect.

## Parser robustness

- Error recovery may leave incomplete indexes until syntax is fixed
- Very large files (> 100k lines) may need debouncing
- LSP assumes UTF-8 source

## Editor / LSP

- UTF-16 positions per LSP spec
- Stdio transport only initially
- No workspace-wide symbol search until include-graph analysis exists

## Reporting issues

Include:

1. Minimal netlist snippet
2. Dialect (Ngspice / LTspice / HSPICE)
3. Expected vs actual diagnostic or hover text

Add a fixture under `test-data/` when fixing.
