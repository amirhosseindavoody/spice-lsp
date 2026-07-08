# Principles

Guiding goals, non-goals, and UX values for spice-lsp. Use these when deciding whether a feature belongs in the MVP or a later phase.

## Goals

1. **Fast feedback while editing** — Diagnostics should feel instant on typical netlists (< 5k lines). Incremental parsing (Tree-sitter) is the foundation.
2. **Works offline** — Single static binary, no cloud services, no simulator required for IDE features.
3. **Dialect-aware but pragmatic** — Support Ngspice, LTspice, and HSPICE over time; ship one dialect correctly before guessing at others.
4. **Editor-agnostic core** — All IDE logic lives in the LSP binary. VS Code is the first client, not the only one.
5. **Testable at every layer** — Parser fixtures, LSP integration tests, and extension smoke tests so regressions are caught in CI.

## Non-goals

| Non-goal | Why |
|----------|-----|
| Running SPICE simulations | Out of scope; use Ngspice/LTspice externally |
| Schematic capture | Netlist text only |
| Netlist ↔ schematic conversion | Different problem domain |
| Replacing simulator error messages | We surface *syntax* and light *semantic* issues before run |
| Perfect formatting on day one | Formatter is post-MVP; MVP focuses on parse + diagnostics |

## UX values

- **Squiggles should be actionable** — Every diagnostic includes a clear message and stable range. Avoid noisy warnings until confidence is high.
- **Don't break partial files** — Users often paste incomplete subcircuits. Parse errors must not prevent indexing the rest of the file.
- **Respect SPICE line continuations** — The `+` continuation character is first-class in the grammar, not an afterthought.
- **Predictable formatting** — When the formatter ships, running it twice must be idempotent (same output).
- **Low configuration** — Sensible defaults; dialect and style options come later via LSP `initializationOptions` or settings.

## MVP scope boundary

The MVP proves the **pipeline works end-to-end in VS Code**:

```
netlist buffer → parse → diagnostics → LSP → VS Code squiggles
```

**In MVP:**

- Syntax diagnostics from Tree-sitter / parse errors
- `textDocument/didOpen`, `didChange`, `didClose` synchronization
- `textDocument/publishDiagnostics`
- VS Code extension: language id, file associations, server spawn

**Explicitly out of MVP:**

- `textDocument/formatting`
- `textDocument/completion`
- `textDocument/definition` / references
- `textDocument/hover`
- Document symbols / outline
- Multi-dialect configuration
- Semantic rules (duplicate component names, undefined models)

Ship MVP, demo it, gather feedback, then expand using [LSP features](5_lsp-features.md) priority order.

## Success criteria for MVP

You can declare MVP done when:

1. `pixi run test` passes parser and LSP integration tests
2. Opening a broken netlist in the VS Code Extension Development Host shows at least one diagnostic
3. Fixing the error clears the diagnostic without restarting the editor
4. A new contributor can follow [MVP guide](development/2_mvp.md) and reach the same result in one session
