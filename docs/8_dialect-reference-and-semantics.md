# Dialect Reference and Net Semantics

Two related capabilities: a **curated dialect reference** the LSP consults for documentation on hover, and **net connectivity analysis** that flags floating nets and dangling nodes. This chapter is the single source of truth for both; other pages link here rather than repeating detail.

Reference-powered hover is **shipped**. Connectivity analysis is **planned**.

---

## Part 1 — Dialect reference library

### Purpose

SPICE dialects differ in directives (`.tran`, `.option`), device syntax, and parameter names. Generic hover text is not enough. spice-lsp ships with — and grows — a **reference corpus you maintain**: structured descriptions of commands, options, element types, and common expressions **per dialect**.

The LSP does not scrape simulator manuals at runtime. It **looks up entries** from checked-in reference data selected by the active dialect.

### What users see

| Cursor on… | Hover shows (from reference) |
|------------|------------------------------|
| `.tran` | Syntax, parameters, units, dialect notes |
| `.option` keyword | Meaning, default, valid values |
| `M` (MOSFET line) | Terminal order, common parameters |
| `{expression}` in `.param` | Allowed functions, unit conventions |

Completion (when implemented) can attach the same entries as `documentation` on completion items.

### Repository layout

```
reference/
├── schema.json              # JSON Schema for reference entries
├── ngspice/
│   ├── directives/
│   │   ├── tran.json
│   │   ├── ac.json
│   │   └── option.json
│   ├── elements/
│   │   ├── R.json
│   │   └── M.json
│   └── expressions.json     # shared {…} expression helpers
├── ltspice/
│   └── …                    # LTspice-specific overrides and additions
└── hspice/
    └── …
```

Author **HSPICE** and **Ngspice** first (HSPICE is the extension default — see [Multi-dialect design](internal/2_multi-dialect-design.md)); add LTspice as the corpus grows. Entries can override or extend `_shared/` where dialects agree.

### Entry format (draft)

Each file describes one construct. Example `reference/ngspice/directives/tran.json`:

```json
{
  "id": "ngspice.directive.tran",
  "kind": "directive",
  "name": ".tran",
  "summary": "Transient analysis",
  "syntax": ".tran Tstep Tstop [Tstart [Tmax]] [UIC]",
  "parameters": [
    { "name": "Tstep", "description": "Suggested printing increment.", "units": "seconds" },
    { "name": "Tstop", "description": "Final time.", "units": "seconds" }
  ],
  "examples": [".tran 1n 100n", ".tran 1u 1m 0 10u UIC"],
  "seeAlso": ["ngspice.directive.options"],
  "dialect": "ngspice"
}
```

The Rust crate `spice-reference` loads and indexes entries by `(dialect, kind, name)`.

### LSP integration

1. Client sends active dialect via `initializationOptions` or `spiceLsp.dialect` setting (default **`hspice`**; command + status bar to switch — [design](internal/2_multi-dialect-design.md)).
2. On `textDocument/hover`, the server maps the cursor CST node to a reference key (e.g. directive name, element letter, option token).
3. Server renders `Hover` markdown from the entry: summary, syntax block, parameter table, examples.
4. Missing entry → no hover (or a one-line fallback from the parse tree). **Gaps are filled by adding reference files**, not hard-coding strings in Rust.

### Authoring workflow

Reference content is **your ongoing work**, independent of parser releases:

1. Add or edit JSON under `reference/<dialect>/` or `reference/_shared/`.
2. Run `pixi run reference-validate` to load and exercise the embedded corpus.
3. Run `pixi run reference-docs` to regenerate the [Dialect reference catalog](reference/README.md).
4. Add or update hover snapshot tests when behavior changes.
5. Ship with the binary (corpus is embedded at compile time via the `spice-reference` build script).

Prefer small, focused files over one giant manual. Link related entries with `seeAlso`.

### Coverage status

| Area | Scope |
|------|-------|
| Shipped | Inline hover from CST + curated `reference/` lookup (HSPICE overlays for `.data` / `.dc` / `.op` and common controls; Ngspice baseline) |
| Growing | Broader dialect coverage; LTspice / remaining HSPICE constructs added incrementally |

---

## Part 2 — Net connectivity analysis

### Purpose

Syntax-correct netlists can still fail simulation because a node is **dangling** (only one connection) or **floating** (no DC path to ground). spice-lsp will analyze connectivity from parsed instance lines and surface these as **semantic diagnostics** (typically warnings).

This is not ERC/DRC and does not replace the simulator — it catches common mistakes early.

### Definitions

| Term | Meaning | Example |
|------|---------|---------|
| **Ground** | Node `0`, `gnd`, `GND`, or dialect-specific ground aliases | `C1 out 0 1u` |
| **Dangling node** | Appears on exactly **one** device terminal in the analyzed scope | Net `bias` only on `R1 in bias` with nothing on `bias` elsewhere |
| **Floating net** | Has connections but **no DC path to ground** through R, L, V, I, or defined grounds | Unconnected island of R–C with no voltage source or ground tie |

Exact rules are dialect-aware (e.g. which nodes count as ground, whether `V` with both nodes internal creates a path). Document rules per dialect in the analyzer and test with fixtures.

### Architecture

```
CST instance lines
       │
       ▼
┌──────────────────┐
│ NetGraph builder │  nodes ↔ terminals, per .subckt scope
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ Connectivity     │  dangling: degree == 1 (exclude intentional probes)
│ passes           │  floating:  no path to ground node set
└────────┬─────────┘
         │
         ▼
   Vec<Diagnostic>  → publishDiagnostics (Warning)
```

Build the graph **per scope**: top level and inside each `.subckt` separately. Subcircuit ports are connections to the parent scope, not isolated graphs.

### Diagnostic examples

| Code | Message | Severity |
|------|---------|----------|
| `spice/dangling-node` | `node 'bias' is connected to only one device terminal` | Warning |
| `spice/floating-net` | `net 'internal' has no DC path to ground` | Warning |

Attach diagnostics to the **node token** on the instance line when possible. Offer a single diagnostic per net, not one per terminal.

### Configuration (planned)

| Setting | Default | Effect |
|---------|---------|--------|
| `spiceLsp.diagnostics.danglingNodes` | `true` | Enable dangling-node pass |
| `spiceLsp.diagnostics.floatingNets` | `true` | Enable floating-net pass |
| `spiceLsp.groundNodes` | `["0", "gnd", "GND"]` | Treat as ground for path search |

### Limitations

- Ignores nets from `.include` files until a cross-file net graph exists (model/subckt include resolution is separate — see [Include and library resolution](9_include-and-lib-resolution.md))
- Ideal voltage sources and shorted nodes need special handling
- Intentionally open probes may false-positive — allow suppress comments or config later

### Status

| Scope | Status |
|-------|--------|
| Duplicate instance names | Shipped (separate from connectivity) |
| Dangling nodes and floating nets (single-file) | Planned |

---

## Testing

| Layer | Test |
|-------|------|
| Reference | Schema validation on all `reference/**/*.json` |
| Reference | Hover snapshot: fixture + cursor → markdown |
| Connectivity | `test-data/semantic/dangling-node.cir` → one `spice/dangling-node` |
| Connectivity | `test-data/semantic/floating-net.cir` → one `spice/floating-net` |
| LSP | Integration test publishes warnings after open |

See [Demo and testing](development/2_demo-and-test.md).

## Related

- [Architecture](4_architecture.md) — crate layout and analysis layers
- [LSP features](5_lsp-features.md) — capability matrix
- [Limitations](7_limitations.md) — what analysis does not cover
- [Design (internal)](internal/1_design.md) — requirements spec
