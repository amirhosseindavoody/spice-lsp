# Large-file / extracted-netlist mode

Design for opening post-layout and other **extracted** netlists (tens to hundreds of MB) in the editor without OOM or multi-second hangs, while keeping full analysis for normal schematic-scale decks.

**Status:** Implemented (size gate, defs-only index, thinned diagnostics, settings). Lazy include materialization and Tree-sitter incremental reuse remain follow-ups.  
**Related:** [Limitations](../7_limitations.md), [Architecture](../4_architecture.md), [Principles](../3_principles.md), [Include and library resolution](../9_include-and-lib-resolution.md), [LSP features](../5_lsp-features.md), [Design](1_design.md).

---

## 1. Problem

spice-lsp today eagerly:

1. Holds the full buffer as a `String`
2. Fully re-parses with Tree-sitter (no incremental `old_tree` reuse yet)
3. Builds an `Index` that records **every instance** plus hierarchical outline children
4. Loads and indexes `.include` / `.lib` targets for definition resolution

That matches interactive schematic netlists (thousands to tens of thousands of lines). It does **not** match extracted dumps (DSPF/SPEF-style or flattened SPICE) where a single open file can be ~300 MB and almost entirely instance lines.

**Full per-instance symbol resolution at that scale is not feasible** with the current model: source + dense owned symbol/outline tables amplify memory to multiple times the file size, and LSP payloads (outline, diagnostics) would overwhelm the client even if the server survived.

Performance targets in [Design](1_design.md) (~50k lines / &lt;100 ms semantic) and [Principles](../3_principles.md) (typical &lt;5k lines) already describe a different operating region.

---

## 2. Lessons from Astral’s ty

[ty](https://github.com/astral-sh/ty) (Astral’s Python type checker / LSP; engine in [astral-sh/ruff](https://github.com/astral-sh/ruff)) is built for **large projects**: millions of lines across many modules, with millisecond incremental updates after edits. Public summary: [language server docs](https://docs.astral.sh/ty/features/language-server/), [announcement](https://astral.sh/blog/ty).

### 2.1 What they do well

| Pattern | Detail |
|---------|--------|
| LSP-first incrementality | Analysis is a Salsa query graph; edits invalidate only dependent queries down to individual definitions |
| Coarse then fine | Parse a whole file once; expensive work (type inference) is per-scope and reusable |
| Lazy dependency work | Skip large parts of third-party code until imports / open files require it |
| Memory pressure control | Drop ASTs after checking and reparse on demand; planned LRU eviction for dominant caches |
| Diagnostic scope | Default `openFilesOnly`; optional workspace diagnostics; prefer pull diagnostics over push-everything |
| Interning | Deduplicate paths and types via the query DB |

### 2.2 What we should not copy wholesale

| ty choice | Why not for spice-lsp (yet) |
|-----------|------------------------------|
| Full Salsa database | Large architectural bet; valuable later for multi-file schematic projects, overkill as the first fix for one huge buffer |
| Definition-level type inference granularity | Extracted SPICE pain is **open/index cost of millions of similar lines**, not “recheck one function” |
| Workspace-scale symbol search as a goal for extracted dumps | Searching millions of `X`/`M`/`R` instance names is rarely useful |

### 2.3 Problem-shape caveat

ty’s “large” usually means **many moderate files** with fine-grained edits. A single ~300 MB netlist is a different stressor (they already see pain on ~28 MB dense stubs). Steal **policy** (lazy, layered, scoped diagnostics, drop heavy IR); do not assume Salsa alone makes full instance indexing viable.

---

## 3. Goals and non-goals

### Goals

| Goal | Detail |
|------|--------|
| Open large extracted files safely | Size/line gate; never build a multi‑GB instance index by default |
| Useful navigation on structure | Go-to-definition for `.subckt` / `.model` / `.param` (local + lazy include/lib) |
| Keep schematic UX unchanged | Files under the threshold keep today’s full index, outline, and diagnostics |
| Bounded LSP payloads | Cap or omit instance outline children; avoid flooding diagnostics |
| Clear mode signaling | Status / log / optional diagnostic so users know analysis is thinned |

### Non-goals (this design)

- Full per-device symbol tables for extracted dumps
- Find-all-references across millions of instances
- Connectivity / net-graph analysis on extracted top-levels
- Formatter rewrites of 100+ MB buffers
- Adopting Salsa in the first implementation slice

---

## 4. Proposed modes

| Mode | When | Index | Outline | Diagnostics | Includes |
|------|------|-------|---------|-------------|----------|
| **Full** (default) | Buffer below threshold | Definitions + instances + refs | Hierarchical, including instances | Current syntax + semantic set | Eager definition graph (current behavior) |
| **Extracted** | Buffer at/above threshold, or user override | **Definitions only** (`.subckt`, `.model`, `.param`) | Structure only (no instance children) | Syntax + cheap checks; skip scans that walk every instance | **Lazy**: resolve on navigation / unknown-model as needed; prefer defs-only indexes for closed files |

Optional third lever later: `spiceLsp.analysisMode = "auto" | "full" | "extracted"` so users can force either side of the gate.

### 4.1 Threshold

Start with a simple gate, e.g.:

- `text.len() >= N` bytes (suggested starting point: **16–32 MiB**), and/or
- line count ≥ **200k**

Exact numbers should be tuned with a fixture and RSS measurements; document the defaults in settings and [Limitations](../7_limitations.md) when implemented.

### 4.2 Feature matrix in extracted mode

| Feature | Behavior |
|---------|----------|
| Syntax diagnostics | Keep (Tree-sitter / line classify); consider debouncing more aggressively |
| Duplicate instance names | **Off** or sampled — full scan is O(instances) |
| Unknown model / subckt | On for references that are checked; may require lazy include lookup of **definition maps only** |
| Document symbols | Subcircuits, models, params; **no** per-instance children |
| Go to definition | Definitions in-file + lazy include/lib |
| Find references | File-local refs among **indexed** symbols only (defs / sparse refs); do not promise all instance hits |
| Hover (reference corpus) | Unchanged (line-local token + corpus) |
| Hover (file-local instance) | Best-effort from the current line without a global instance table |
| Completion / formatter / connectivity | Remain out of scope or explicitly disabled on huge buffers |

---

## 5. Architecture sketch

Keep the existing crate split; add a **policy layer** in analysis rather than a second parser.

```
didOpen / didChange
  → choose AnalysisProfile { Full, Extracted }
  → parse / classify lines (shared)
  → build_index(profile)
       Full:      today’s symbols + outline
       Extracted: definitions (+ optional sparse model refs), thin outline
  → includes
       Full:      eager resolve (current)
       Extracted: stub graph; materialize IncludedFile defs on demand
  → publish diagnostics / serve navigation
```

### 5.1 Index changes (`spice-parser`)

- Extend `build_index` (or wrap it) with a profile flag:
  - Skip `SymbolKind::Instance` insertion
  - Skip pushing instances into `document_symbols` children
  - Optionally still record **model/subckt name references** from instance lines into the references map **without** storing an instance `Symbol` per line (supports unknown-model + goto without GB-scale vectors)
- Prefer interned or borrowed name keys where practical later; first slice can keep `String` keys if instance rows are omitted

### 5.2 Include graph (`spice-parser` / LSP)

- In extracted mode, do not retain full text + full `Index` for every include by default
- Materialize definition maps when resolving go-to-definition or unknown-model
- Cap concurrent materialized includes; depth cap already exists (`DEFAULT_MAX_INCLUDE_DEPTH`)

### 5.3 LSP backend (`spice-lsp`)

- Gate on `didOpen` / after large `didChange` (rare for extracted files)
- Avoid cloning entire workspace buffers into analyze snapshots when possible
- Prefer pull diagnostics later; until then, publish a **small** diagnostic set in extracted mode
- Log once: `spice-lsp: extracted analysis mode (N bytes) — instance indexing disabled`
- Cancel or `spawn_blocking` long analyzes so the server stays responsive

### 5.4 Easy wins independent of mode

Worth doing even for schematic files (aligned with ty’s “don’t redo work”):

| Win | Notes |
|-----|-------|
| Deduplicate parses in `analyze_with_includes` | Root is parsed multiple times today |
| Persist Tree-sitter `Tree` + `InputEdit` | Matches documented incremental architecture |
| Line index / rope for UTF-16 ↔ byte mapping | Avoid O(file) scans per symbol conversion |
| Cap outline size | Even in full mode, enormous outlines are hostile to editors |

---

## 6. Comparison: ty patterns → spice-lsp actions

| ty pattern | spice-lsp action |
|------------|------------------|
| Fine-grained incremental queries | Near term: cache parse/index per document; invalidate on change. Later: optional query-style layering if multi-file cost dominates |
| Skip irrelevant dependencies | Lazy `.include` / `.lib` in extracted mode |
| Drop AST after use | Already drop Tree; extend to “don’t retain instance IR” |
| `openFilesOnly` / pull diagnostics | Thin diagnostic set + eventual pull; never push millions of warnings |
| Coarse parse, fine semantics | Line classify always; instance indexing and connectivity optional |

---

## 7. Implementation slices

| Slice | Status |
|-------|--------|
| 1. Gate + profile plumbing (`AnalysisMode` / `AnalysisProfile`, threshold) | Done |
| 2. Defs-only index + outline without instances; goto via line classify | Done |
| 3. Diagnostic thinning (no duplicate-name; sparse unknown-model) | Done |
| 4. Settings (`spiceLsp.analysisMode`, `extractedByteThreshold`) + docs | Done |
| 5. Defs-only indexes for includes when root is extracted / include is huge | Done |
| 6. Lazy include materialization (load on demand only) | Open |
| 7. Perf hygiene — single-parse path, incremental Tree-sitter, position index | Open |

Public docs: [LSP features](../5_lsp-features.md), [Limitations](../7_limitations.md), VS Code README.

---

## 8. Testing

| Layer | Approach |
|-------|----------|
| Unit | `build_index` with `Extracted` on a small fixture asserting zero instance symbols and retained `.subckt`/`.model` |
| Threshold | Force profile via test API or setting without needing a 300 MB file in CI |
| Integration | LSP: open “large” synthetic buffer; assert outline has no instance flood; goto def still works |
| Manual / bench | Optional local 100–300 MB extracted file: RSS + time-to-first-navigation; not required in CI |

Do not check multi‑hundred‑MB binaries into the repo.

---

## 9. Open questions

1. **Default threshold** — bytes vs lines vs both; different defaults for `.spf`/`.net` associations?
2. **Model refs without instance symbols** — enough for unknown-model quality, or accept weaker checks in extracted mode?
3. **Editor still holds 300 MB** — server-side wins don’t fix VS Code memory; document that opening such files is inherently heavy
4. **Partial / viewport analysis** — analyze only visible ranges later? Powerful but much more complex; not in the first slices
5. **Salsa later?** — revisit if multi-file schematic workspaces (many includes, frequent edits) become the bottleneck after extracted mode lands

---

## 10. Success criteria

When implemented, an extracted ~100–300 MB netlist should:

1. Open without process kill / multi‑GB RSS from instance indexes
2. Show a usable structural outline (subcircuits/models/params)
3. Support go-to-definition for models/subcircuits with lazy includes
4. Leave schematic-scale files behaviorally unchanged
5. Document the mode and limits in [Limitations](../7_limitations.md)
