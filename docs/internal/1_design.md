# SPICE Netlist LSP and Formatter Design Document

## 1. Executive Summary

This document defines the system design, capabilities, and requirements for a Language Server Protocol (LSP) and Formatter tailored for SPICE (Simulation Program with Integrated Circuit Emphasis) netlists. The goal is to improve developer velocity, reduce syntax errors, and enforce stylistic consistency across analog and mixed-signal simulation workflows.

---

## 2. System Capabilities

### 2.1 Language Server Protocol (LSP) Capabilities

The LSP server implements the following capabilities to provide real-time IDE feedback:

- **Syntax and Semantic Diagnostics:**
    - **MVP / syntax:** Missing `.ends`, bad line continuations, parse errors.
    - **v0.2:** Duplicate component identifiers, undefined model/subcircuit references.
    - **v0.5 / connectivity:** Dangling nodes (single terminal connection) and floating nets (no DC path to ground). Severity warning; configurable. See [Dialect reference and net semantics](../8_dialect-reference-and-semantics.md).
- **Navigation (Go to Definition & Find References):**
    - Resolve references for subcircuits (`.subckt`) and models (`.model`).
    - Map parameter definitions (`.param`) to their usages in expressions.
- **Autocomplete and Snippets:**
    - Offer context-aware suggestions for basic elements (R, C, L, diodes, transistors).
    - Provide templates for simulation directives (e.g., `.tran`, `.ac`, `.dc`, `.temp`).
- **Hover Documentation:**
    - **v0.3 (file-local):** Subcircuit pin order, in-file model parameters.
    - **v0.5 (dialect reference):** Curated documentation for directives (`.tran`, `.ac`), `.option` keywords, element types, and common expressions — **authored per dialect** in a `reference/` corpus the LSP loads at runtime, not hard-coded in server logic. Coverage grows over time as you add entries for Ngspice, LTspice, and HSPICE.
- **Document Outline (Symbols):**
    - Index hierarchical structures, isolating `.subckt` blocks, `.model` definitions, and control blocks.

### 2.2 Formatter Capabilities

The formatting engine processes netlist files to enforce consistent layouts:

- **Columnar Alignment:** Align component names, nodes, model references, values, and parameters in tabular columns.
- **Case Normalization:** Enforce uppercase, lowercase, or camelCase formatting for keywords, control options, and unit suffixes.
- **Continuation-Line Standardization:** Format multi-line statements wrapped with the `+` character using predictable indentation.
- **Comment Block Structuring:** Standardize inline comments (using `;` or `$`) and line-start comments (`*`).

---

## 3. System Requirements

### 3.1 Functional Requirements

- **Dialect Support:** The parser must support standard SPICE variants, specifically LTspice, Ngspice, and HSPICE syntax.
- **Performance:** Code diagnostics must execute in under 100ms on files up to 50,000 lines.
- **Robustness:** The parser must gracefully recover from syntax errors to continue indexing subsequent parts of the file.

### 3.2 Technical & Architectural Requirements

- **Parser Technology:** Implement the parser using a formal grammar parser-generator like Tree-sitter. This ensures incremental parsing capability for low-latency editing.
- **Communication Protocol:** Conform strictly to the official LSP specification (JSON-RPC 2.0).
- **Distribution:** Package the LSP as a standalone executable (compiled Go or Rust) with no external runtimes required.

---

## 4. MVP Strategy (Ship Before Full Feature Set)

The full capability list in sections 2–3 is the **north star**. The first deliverable is a narrow **MVP** that proves the pipeline in VS Code before investing in navigation, completion, or formatting.

### 4.0.1 MVP definition

**In scope:**

- Rust workspace: `spice-parser` + `spice-lsp` binary
- Tree-sitter grammar for a **single dialect** (Ngspice first)
- Syntax diagnostics only (parse errors, unclosed `.subckt`)
- LSP: `initialize`, text document sync, `publishDiagnostics`
- VS Code extension that spawns the binary over stdio

**Out of scope for MVP:**

- Formatter, completion, go-to-definition, references
- Dialect reference corpus and reference-powered hover
- Floating-net / dangling-node analysis
- Multi-dialect reference namespaces and `.include` resolution

### 4.0.2 MVP milestones

| # | Milestone | Verification |
|---|-----------|--------------|
| M1 | Cargo workspace + pixi tasks | `pixi run cargo build` |
| M2 | Minimal Tree-sitter grammar | Corpus / fixture parse tests |
| M3 | Parser → diagnostics API | `pixi run cargo test -p spice-parser` |
| M4 | tower-lsp stdio server | LSP integration test |
| M5 | `test-data/` fixtures | CI green on `pixi run test` |
| M6 | VS Code extension | F5 → squiggles on invalid netlist |
| M7 | Documented demo script | [Demo and testing](../development/3_demo-and-test.md) |

Detailed steps: [MVP guide](../development/2_mvp.md).

### 4.0.3 Demo and test strategy

| Layer | Method |
|-------|--------|
| Grammar | Tree-sitter corpus + Rust fixture tests |
| Parser | Golden diagnostics on `test-data/invalid/*` |
| LSP | JSON-RPC harness over stdio (subprocess or mock client) |
| VS Code | Extension Development Host (F5), Problems panel |
| CI | `pixi install && pixi run test` on every push |

Manual smoke: open `test-data/invalid/unclosed-subckt.cir`, fix `.ends`, confirm diagnostic clears.

### 4.0.4 VS Code as primary client

Distribution path:

1. **Development:** `spiceLsp.serverPath` points at `target/debug/spice-lsp`
2. **Early adopters:** side-load `.vsix` built with `vsce package`
3. **General availability:** Marketplace publish with platform-specific binary download or bundle

Extension architecture (thin Node client, Rust server): [VS Code integration](../development/4_vscode-integration.md).

Post-MVP features roll out in phases documented in [Architecture](../4_architecture.md) and [LSP features](../5_lsp-features.md). Deep semantics (reference library + net connectivity) are specified in [Dialect reference and net semantics](../8_dialect-reference-and-semantics.md). Multi-dialect selection, corpus authoring, and hover reuse are specified in [Multi-dialect support](2_multi-dialect-design.md).

### 4.0.5 Post-MVP roadmap (summary)

| Phase | Focus |
|-------|-------|
| v0.2 | Symbol index, navigation, duplicate/undefined warnings |
| v0.3 | Completion, file-local hover |
| v0.4 | Formatter, dialect setting |
| v0.5 | Curated dialect reference → hover; dangling-node and floating-net diagnostics |

---

## 5. System Architecture & Implementation

### 5.1 Implementation Language

The LSP and formatter will be implemented in **Rust** to satisfy the low-latency and performance requirements (<100ms on 50k lines) while guaranteeing thread safety and memory efficiency without a garbage collector.

### 5.2 Architecture & Design

The system uses a classic compiler frontend architecture integrated into an event-driven JSON-RPC server:

```
[IDE Client] 
     │  (LSP over JSON-RPC 2.0 via StdIO)
     ▼
┌────────────────────────────────────────────────────────┐
│ LSP Server (tower-lsp)                                 │
│    │                                                   │
│    ├─► [Parser Engine] ─────────────────────────────┐   │
│    │   Incrementally parses buffer into Tree-sitter │   │
│    │   Concrete Syntax Tree (CST)                   │   │
│    │                                                │   │
│    ├─► [Diagnostics Analyzer] ◄─────────────────────┘   │
│    │   Syntax (MVP), symbols (v0.2), connectivity (v0.5)│
│    │                                                   │
│    ├─► [Reference Index] ◄── reference/<dialect>/     │
│    │   v0.5: hover docs for directives, options, elems │
│    │                                                   │
│    └─► [Formatter Engine] ◄─────────────────────────┘   │
│        v0.4: columnar alignment & continuation formatting│
└────────────────────────────────────────────────────────┘
```

- **LSP Layer:** Handles connection lifecycle, text document synchronization, and capability routing.
- **Incremental Parsing:** Tree-sitter maintains an active syntax tree; edits re-parse only changed ranges.
- **Reference Index (v0.5):** Loads structured JSON entries from `reference/` per active dialect; powers `textDocument/hover` and enriches completion documentation. Maintained manually over time — see [Dialect reference and net semantics](../8_dialect-reference-and-semantics.md).
- **Net Graph (v0.5):** Builds terminal connectivity from instance lines; emits dangling-node and floating-net warnings.
- **Formatting Pipeline (v0.4):** CST → column rules → `TextEdit` actions.

### 5.3 Key Dependencies

- **`tower-lsp`**: High-level LSP implementation framework for Rust built on Tokio.
- **`tree-sitter`**: Rust bindings to the incremental parsing library.
- **`tree-sitter-spice`**: Custom or community grammar for parsing SPICE dialects.
- **`serde` / `serde_json`**: Serialization and deserialization of LSP messages.
- **`clap`**: Robust command-line argument parser for standalone formatter CLI execution.
