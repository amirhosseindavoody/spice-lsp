# Principles

Goals, non-goals, and UX values for spice-lsp.

## What good looks like

A developer editing a netlist in VS Code should get:

1. **Immediate syntax and semantic feedback** — parse errors, duplicate names, unknown models
2. **Jump to definitions and a useful outline** — subcircuits, models, parameters
3. **Dialect-aware documentation on hover** — curated reference plus file-local pin/model detail
4. **Include-aware analysis** — `.include` / `.lib` participate in checks and navigation
5. **Consistent formatting and completion** (planned) — align netlists and suggest elements/directives
6. **Connectivity warnings** (planned) — dangling nodes and floating nets before simulation

Details on reference hover and connectivity: [Dialect reference and net semantics](8_dialect-reference-and-semantics.md).

## Goals

1. **Fast feedback while editing** — Diagnostics feel instant on typical netlists (< 5k lines). Tree-sitter incremental parsing is the foundation.
2. **Works offline** — Single static binary; no cloud services; no simulator required for IDE features.
3. **Dialect-aware, corpus-driven docs** — Ngspice, LTspice, and HSPICE differ. Hover (and later completion) documentation come from a **curated reference library maintained per dialect**, not hard-coded strings scattered in Rust.
4. **Catch connectivity mistakes before simulation** — Flag dangling nodes and floating nets as warnings when analysis is confident enough.
5. **Editor-agnostic core** — All language logic lives in the LSP binary. VS Code is the first client, not the only one.
6. **Testable at every layer** — Parser fixtures, reference schema tests, hover snapshots, and LSP integration tests in CI.

## Non-goals

| Non-goal | Why |
|----------|-----|
| Running SPICE simulations | Use Ngspice/LTspice externally |
| Schematic capture | Netlist text only |
| Auto-generating reference from PDF manuals | You author `reference/` deliberately; quality over coverage |
| Full ERC/DRC | Floating-net checks are heuristic helpers, not sign-off tools |
| Replacing simulator errors | We front-load syntax and common semantic mistakes |

## UX values

- **Actionable squiggles** — Clear message, stable range, stable diagnostic code (e.g. `spice/floating-net`).
- **Graceful partial files** — Incomplete subcircuits during editing must not block analysis of the rest of the buffer.
- **Respect line continuations** — The `+` character is first-class in the grammar; HSPICE `.DATA` value rows may also continue without `+`.
- **Documentation you trust** — Reference hover reads like a concise manual entry: syntax, units, examples. Missing entries show nothing rather than wrong text.
- **Warn, don’t nag** — Connectivity warnings are severity `Warning`, configurable, and scoped to reduce false positives on intentional open nodes.
- **Low configuration** — Sensible defaults; dialect and diagnostics toggles via settings when needed.

## Success criteria

1. `pixi run test` passes parser and LSP integration tests
2. Invalid netlist in the editor shows a syntax diagnostic; fixing it clears the diagnostic without restart
3. Go to definition reaches `.model` / `.subckt` across `.include` / `.lib` when paths resolve
4. Hover on a documented directive shows dialect reference text for the active dialect
5. A contributor can follow [Demo and testing](development/2_demo-and-test.md) and reproduce the smoke demo
