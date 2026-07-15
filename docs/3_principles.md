# Principles

Goals, non-goals, and UX values for spice-lsp. Use this page to decide whether work belongs in MVP or a later phase.

## North-star experience

A developer editing a netlist in VS Code should get:

1. **Immediate syntax feedback** (MVP)
2. **Jump to definitions and a useful outline** (v0.2)
3. **Completion and quick in-file hover** (v0.3)
4. **Consistent formatting and dialect selection** (v0.4)
5. **Authoritative dialect documentation on hover and warnings on bad connectivity** (v0.5)

Steps 1–4 build the pipeline; step 5 is where the tool becomes a SPICE-aware assistant rather than a generic syntax checker. Details: [Dialect reference and net semantics](8_dialect-reference-and-semantics.md).

## Goals

1. **Fast feedback while editing** — Diagnostics feel instant on typical netlists (< 5k lines). Tree-sitter incremental parsing is the foundation.
2. **Works offline** — Single static binary; no cloud services; no simulator required for IDE features.
3. **Dialect-aware, corpus-driven docs** — Ngspice, LTspice, and HSPICE differ. Long term, hover and completion documentation come from a **curated reference library you maintain per dialect**, not hard-coded strings scattered in Rust.
4. **Catch connectivity mistakes before simulation** — Flag dangling nodes and floating nets as warnings when analysis is confident enough (v0.5).
5. **Editor-agnostic core** — All language logic lives in the LSP binary. VS Code is the first client, not the only one.
6. **Testable at every layer** — Parser fixtures, reference schema tests, hover snapshots, and LSP integration tests in CI.

## Non-goals

| Non-goal | Why |
|----------|-----|
| Running SPICE simulations | Use Ngspice/LTspice externally |
| Schematic capture | Netlist text only |
| Auto-generating reference from PDF manuals | You author `reference/` deliberately; quality over coverage on day one |
| Full ERC/DRC | Floating-net checks are heuristic helpers, not sign-off tools |
| Replacing simulator errors | We front-load syntax and common semantic mistakes |

## UX values

- **Actionable squiggles** — Clear message, stable range, stable diagnostic code (e.g. `spice/floating-net`).
- **Graceful partial files** — Incomplete subcircuits during editing must not block analysis of the rest of the buffer.
- **Respect line continuations** — The `+` character is first-class in the grammar; HSPICE `.DATA` value rows may also continue without `+`.
- **Documentation you trust** — Reference hover reads like a concise manual entry: syntax, units, examples. Missing entries show nothing rather than wrong text.
- **Warn, don’t nag** — Connectivity warnings are severity `Warning`, configurable, and scoped to reduce false positives on intentional open nodes.
- **Low configuration** — Sensible defaults; dialect and diagnostics toggles via settings when needed.

## MVP scope boundary

MVP proves the **editor pipeline** only:

```
netlist buffer → parse → syntax diagnostics → LSP → VS Code squiggles
```

**In MVP:** syntax diagnostics, text sync, `publishDiagnostics`, VS Code extension.

**Not in MVP** (including v0.5 goals):

| Deferred | Target phase |
|----------|--------------|
| Dialect reference hover | v0.5 |
| Floating / dangling node analysis | v0.5 |
| Completion, file-local hover | v0.3 |
| Navigation, outline | v0.2 |
| Formatter | v0.4 |

Ship MVP first, then follow the phase order in [Architecture](4_architecture.md).

## Success criteria for MVP

1. `pixi run test` passes parser and LSP integration tests
2. Invalid netlist in the Extension Development Host shows a syntax diagnostic
3. Fixing the error clears the diagnostic without restarting the editor
4. A contributor can follow [Demo and testing](development/2_demo-and-test.md) and reproduce the smoke demo

Success criteria for **v0.5** (future): hover on `.tran` shows Ngspice reference text; `test-data/semantic/dangling-node.cir` produces `spice/dangling-node`.
