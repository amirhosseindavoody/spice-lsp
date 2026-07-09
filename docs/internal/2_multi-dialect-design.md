# Multi-dialect support design

Design for [issue #16](https://github.com/amirhosseindavoody/spice-lsp/issues/16): selectable SPICE dialects (default **HSPICE**), retained **Ngspice** support, a maintainable system for growing syntax/reference knowledge, and reuse of that data for hover (and later completion).

**Status:** design only — no implementation in this change.  
**Related:** [Dialect reference and net semantics](../8_dialect-reference-and-semantics.md), [LSP features](../5_lsp-features.md), [Architecture](../4_architecture.md).

---

## 1. Goals and non-goals

### Goals

| Goal | Detail |
|------|--------|
| User-selectable dialect | VS Code setting + Command Palette command |
| Default = HSPICE | Matches issue #16; Ngspice remains fully supported |
| Shared knowledge system | One corpus drives diagnostics policy, hover, and (later) completion docs |
| Low-friction authoring | Adding a directive/element/rule is mostly data + a test, not scattered Rust strings |
| Keep current Ngspice behavior | Existing fixtures and diagnostics stay green under `ngspice` |

### Non-goals (this design / first implementation slices)

- Full HSPICE / LTspice grammar parity on day one
- Scraping simulator manuals at runtime (bash-lsp `man` / explainshell style)
- Per-file dialect auto-detection from content (may come later as a hint)
- Formatter dialect profiles (still v0.4+)
- Connectivity analysis (still v0.5; dialect-agnostic graph with dialect-specific ground aliases later)

---

## 2. Lessons from reference systems

### 2.1 Ruff ([astral-sh/ruff](https://github.com/astral-sh/ruff), [rules docs](https://docs.astral.sh/ruff/rules/))

**What they do well**

- **Single source of truth next to behavior:** rule docs live as structured `///` sections on the violation type; `cargo dev generate-docs` projects them to Markdown for the public site.
- **Registration table + codegen:** `codes.rs` + proc macros produce the `Rule` enum and metadata accessors so “forgot to register” fails loudly.
- **CI gates:** `generate-all --mode check` and `check_docs_formatted.py` reject missing sections / stale generated output.
- **Stable IDs + human names:** codes and kebab-case names with redirects.

**What we should not copy wholesale**

- Embedding long simulator-manual prose in Rust doc comments (wrong medium for SPICE).
- A giant proc-macro registry for hundreds of rules before we need it.
- MkDocs Material as a second doc site unless we later publish a public “reference catalog.”

**Takeaway for spice-lsp:** treat **checked-in structured data** as the SSOT (like Ruff treats rule metadata), generate indexes / book pages / Rust embeds from it, and **CI-check** that generated artifacts match.

### 2.2 bash-language-server ([bash-lsp/bash-language-server](https://github.com/bash-lsp/bash-language-server))

**What they do well**

- **Layered hover:** optional rich external docs (explainshell) → shell `help`/`man` → file-local symbol comments.
- **Markdown LSP contract:** hover is always `MarkupContent` markdown.
- **Memoization** of expensive doc lookups.
- **Opt-in external services** (explainshell off by default).

**What we should not copy**

- Runtime `man` / network scrape as the **primary** SPICE reference (manuals are not on `man`, dialects diverge).
- Detecting a “dialect” (shebang) without switching documentation corpora.
- Letting external docs **replace** file-local hover instead of stacking with it.

**Takeaway for spice-lsp:** keep a **priority chain** for hover (reference corpus → file-local CST → nothing), cache the corpus at startup, never require an external service for basic tips.

---

## 3. Product behavior

### 3.1 Dialects

| Id | Label | Initial role |
|----|-------|--------------|
| `hspice` | HSPICE | **Default** |
| `ngspice` | Ngspice | Current parser/diagnostics baseline; keep working |
| `ltspice` | LTspice | Stub corpus + setting value; grammar/rules grow later |

Unknown dialect values → error diagnostic on initialize / config change, fall back to `hspice` with a logged warning.

### 3.2 How the user chooses

1. **Setting:** `spiceLsp.dialect` — enum `hspice` \| `ngspice` \| `ltspice`, default **`hspice`**.
2. **Command:** `SPICE LSP: Set Dialect…` — QuickPick; writes the setting (workspace if a folder is open, else user) and restarts / notifies the server.
3. **Status bar** (recommended in the same slice): show current dialect; click opens the QuickPick.

Optional later (not required for #16):

- `# spice-lsp dialect=hspice` file header / `.spice-lsp.toml`
- Infer from path heuristics (`*.sp` in an HSPICE tree) as a *suggestion* only

### 3.3 Client ↔ server contract

```text
initialize.initializationOptions.dialect  →  "hspice" | "ngspice" | "ltspice"
workspace/didChangeConfiguration          →  spiceLsp.dialect
```

Extension always sends the resolved dialect on start and on change. Server stores it per-session (workspace-wide for v1; per-document overrides later).

Changing dialect:

1. Re-analyze all open documents with the new dialect profile.
2. Republish diagnostics.
3. Clear hover/completion caches keyed by dialect.

---

## 4. Architecture: one corpus, many consumers

```
                    ┌─────────────────────────────────────┐
                    │  reference/  (SSOT, authored data)  │
                    │  schema + per-dialect entries       │
                    └──────────────┬──────────────────────┘
                                   │
              pixi run reference-codegen / validate
                                   │
           ┌───────────────────────┼───────────────────────┐
           ▼                       ▼                       ▼
   spice-reference crate    docs book pages         CI snapshots
   (embedded index)         (optional catalog)      (hover / schema)
           │
           ▼
   spice-parser  ←── DialectProfile (syntax flags, comment rules, …)
           │
           ▼
   spice-lsp  (diagnostics, hover, later completion)
           │
           ▼
   VS Code extension (setting, command, status bar)
```

**Principle:** Rust implements *mechanisms* (parse, index, lookup, render). Humans author *knowledge* as data under `reference/`. Parser dialect quirks that cannot be expressed as data yet live in a small `DialectProfile` table in Rust, keyed by the same dialect ids.

---

## 5. Reference corpus (Ruff-inspired authoring)

### 5.1 Layout

Evolve the plan in [§ Dialect reference](../8_dialect-reference-and-semantics.md) with an explicit **shared + override** model:

```
reference/
├── schema.json                 # JSON Schema for entries
├── _shared/                    # constructs common across dialects
│   ├── directives/
│   │   └── subckt.json
│   └── elements/
│       └── R.json
├── hspice/
│   ├── dialect.toml            # metadata: display name, aliases, comment styles
│   ├── directives/
│   │   └── option.json         # HSPICE-specific or override
│   └── elements/
├── ngspice/
│   ├── dialect.toml
│   └── …
└── ltspice/
    ├── dialect.toml
    └── …
```

### 5.2 Entry shape (v1)

```json
{
  "id": "hspice.directive.tran",
  "kind": "directive",
  "name": ".tran",
  "summary": "Transient analysis",
  "syntax": ".TRAN tstep tstop [tstart [tmax]] [UIC]",
  "parameters": [
    { "name": "tstep", "description": "Printing / sampling step.", "units": "s" }
  ],
  "examples": [".TRAN 1p 10n"],
  "seeAlso": ["hspice.directive.option"],
  "diagnostics": ["spice/unknown-directive"],
  "since": "0.3.0",
  "dialectNotes": "HSPICE accepts …"
}
```

Required sections (CI-enforced, Ruff-style): `id`, `kind`, `name`, `summary`, `syntax`.  
Optional: `parameters`, `examples`, `seeAlso`, `diagnostics`, `deprecated`, `dialectNotes`.

### 5.3 Merge rules

1. Load `_shared/` as base for the active dialect.
2. Overlay `reference/<dialect>/` by `id` / `(kind, name)` — dialect file wins.
3. Missing entry → no hover / no completion doc (not an error). Gaps are filled by adding JSON.

### 5.4 Codegen / validation tasks

| Task | Purpose |
|------|---------|
| `pixi run reference-validate` | JSON Schema + required sections + unique ids |
| `pixi run reference-codegen` | Emit Rust `phf`/static index or `include_dir!` manifest |
| `pixi run reference-check` | CI: codegen dry-run / diff (Ruff `Mode::Check`) |
| Optional later | Generate mdBook catalog pages from the same JSON |

**Authoring workflow (add a new directive):**

1. Add/edit `reference/<dialect>/directives/foo.json` (or `_shared/` if universal).
2. `pixi run reference-validate`.
3. Add hover snapshot fixture under `test-data/hover/<dialect>/`.
4. `pixi run test`.
5. No Rust change unless a new *kind* or lookup path is needed.

This is the spice-lsp analogue of Ruff’s “add rule → docs fall out of metadata,” with **JSON as the authoring surface** instead of `///` comments.

---

## 6. DialectProfile (syntax / semantics knobs)

Until grammars fully diverge, keep a Rust profile beside the corpus:

```rust
struct DialectProfile {
    id: DialectId,
    // Comments recognized for toggle / highlighting hints
    line_comment_prefixes: &'static [&'static str], // e.g. hspice: ["*"], ngspice: ["*", ";", "$"]
    // Directives treated as unknown → warning vs ignore
    unknown_directive_severity: Severity,
    // Element letter sets, continuation rules, case sensitivity, …
}
```

**v1 behavior**

| Concern | `hspice` | `ngspice` | `ltspice` |
|---------|----------|-----------|-----------|
| Parse grammar | Current line-oriented grammar (shared) | Same | Same |
| Comment styles (docs / future toggle) | `*` primary | `*`, `;`, `$` | `$` / `*` (document; refine later) |
| Semantic diagnostics | Same engines; corpus may gate “unknown directive” lists | Current fixtures | Minimal |
| Hover | `hspice` corpus (+ `_shared`) | `ngspice` corpus | `ltspice` stub |

**Later:** dialect-specific Tree-sitter grammars or grammar injections only when shared tokens are insufficient (do not fork three full grammars prematurely).

---

## 7. Hover design (bash-lsp layering + corpus)

### 7.1 Resolution order

```
cursor token
  1. dialect reference lookup (kind + name + active dialect)
  2. file-local hover (subckt pins, in-file .model / .param)     // v0.3 slice
  3. null
```

Never call out to the network. Render markdown:

```markdown
### `.tran` — Transient analysis
**Dialect:** HSPICE

```
.TRAN tstep tstop [tstart [tmax]] [UIC]
```

| Parameter | Description | Units |
|-----------|-------------|-------|
| tstep | … | s |

**Examples**
- `.TRAN 1p 10n`
```

### 7.2 Mapping cursor → entry

1. Classify line / token: directive name, element type letter, `.option` keyword, etc. (reuse / extend CST + symbol index).
2. Build key `(dialect, kind, normalized_name)`.
3. Lookup in embedded index; try dialect overlay then `_shared`.

### 7.3 Same data for completion (follow-on)

Completion items attach `documentation` from the same entry. No parallel doc strings in Rust.

---

## 8. VS Code extension changes

| Item | Change |
|------|--------|
| `package.json` settings | `spiceLsp.dialect` enum, default `hspice` |
| Command | `spiceLsp.setDialect` → QuickPick |
| Status bar | `HSPICE` / `Ngspice` / `LTspice` |
| `LanguageClient` init | `initializationOptions: { dialect }` |
| Middleware / config listener | On dialect change → `DidChangeConfiguration` + optional restart if needed |
| Marketplace README | Document default HSPICE; how to switch to Ngspice |

TextMate grammar stays shared initially; dialect-specific highlighting can wait.

---

## 9. Phased delivery

### Phase A — Dialect switch plumbing (unblocks #16 UX)

- Setting + command + status bar; default **hspice**.
- Server accepts dialect; re-analyzes on change.
- `DialectProfile` stub; **behavior still matches today’s Ngspice parser** for both `hspice` and `ngspice` except profile metadata / empty HSPICE corpus.
- Docs: default dialect, how to switch.

**Exit criteria:** user can switch dialects; Ngspice fixtures pass with `spiceLsp.dialect=ngspice`; HSPICE default does not break open/diagnostics smoke tests.

### Phase B — Reference crate + hover

- Stand up `reference/` schema + `_shared` + starter `hspice` / `ngspice` entries (small set: `.subckt`, `.ends`, `.model`, `.param`, `.tran`, `R`, `C`, `X`).
- `spice-reference` crate + validate/codegen pixi tasks.
- Implement `textDocument/hover` with layered resolution.
- Snapshot tests per dialect.

### Phase C — Dialect-sensitive diagnostics / grammar

- Unknown-directive / option lists from corpus.
- Comment / continuation profile differences.
- Split grammar only where needed; grow LTspice.

### Phase D — Catalog docs (optional)

- Generate a book chapter or static catalog from JSON (Ruff `generate-docs` analogue), still one SSOT.

Issue #16 is satisfied by **Phase A + a clear path through B**; B can ship in the same epic as follow-up PRs.

---

## 10. Testing strategy

| Layer | Tests |
|-------|-------|
| Schema | Every JSON entry validates; required sections present |
| Merge | Overlay wins; shared fallback works |
| LSP | `initialize` with dialect; `didChangeConfiguration` republishes |
| Hover | Fixtures per dialect; missing entry → null |
| Regression | Existing Ngspice stdio tests run with explicit `ngspice` |
| Extension | Setting default is `hspice`; command updates config (smoke / manual) |

---

## 11. Risks and decisions

| Topic | Decision |
|-------|----------|
| Default dialect | **HSPICE** per #16 (overrides earlier docs that said Ngspice default) |
| One grammar vs many | **One shared grammar in Phase A–B**; profile flags first |
| Doc authoring medium | **JSON under `reference/`**, not Rust comments |
| External doc services | **Out of scope**; optional later, opt-in only |
| LTspice | Enum + stub corpus early; deep support later |
| Breaking change | Default dialect change may surprise Ngspice users — document prominently; one-click switch |

---

## 12. Open questions (resolve during Phase A implementation)

1. Should dialect be **workspace-only** or allow **per-file** override in v1?
2. Do we embed the full corpus in the binary, or load from an extension-relative path for faster iteration?
3. Minimum HSPICE starter set for Phase B (which directives matter first for the author’s flows)?
4. Status bar vs Command Palette only for v1 UX?

---

## 13. Implementation checklist (when coding starts)

- [ ] `spiceLsp.dialect` + `spiceLsp.setDialect` + status bar
- [ ] Server session dialect + config update path
- [ ] `DialectProfile` + Ngspice parity tests under `ngspice`
- [ ] Update [LSP features](../5_lsp-features.md), [limitations](../7_limitations.md), Marketplace README for default HSPICE
- [ ] Scaffold `reference/schema.json`, `_shared/`, `hspice/`, `ngspice/`
- [ ] `spice-reference` + validate/codegen tasks
- [ ] Hover provider + snapshots
- [ ] Close #16 when Phase A is shipped and Phase B is scheduled/linked

---

## 14. References

- Issue: https://github.com/amirhosseindavoody/spice-lsp/issues/16
- Ruff rules: https://docs.astral.sh/ruff/rules/
- Ruff repo (docs codegen): `crates/ruff_dev/src/generate_docs.rs`, `scripts/generate_mkdocs.py`, `CONTRIBUTING.md` (“Adding a new rule”)
- bash-language-server hover: `server/src/server.ts`, `server/src/util/sh.ts`, `server/src/analyser.ts` (explainshell)
- Existing spice-lsp plan: [Dialect reference and net semantics](../8_dialect-reference-and-semantics.md)
