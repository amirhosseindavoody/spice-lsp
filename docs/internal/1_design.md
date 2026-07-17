# SPICE Netlist LSP and Formatter Design Document

## 1. Executive Summary

This document defines the system design, capabilities, and requirements for a Language Server Protocol (LSP) and Formatter tailored for SPICE (Simulation Program with Integrated Circuit Emphasis) netlists. The goal is to improve developer velocity, reduce syntax errors, and enforce stylistic consistency across analog and mixed-signal simulation workflows.

---

## 2. System Capabilities

### 2.1 Language Server Protocol (LSP) Capabilities

The LSP server implements the following capabilities to provide real-time IDE feedback:

- **Syntax and Semantic Diagnostics:**
    - **Syntax:** Missing `.ends`, bad line continuations, parse errors.
    - **Symbols:** Duplicate component identifiers, undefined model/subcircuit references; include/lib path issues.
    - **Connectivity (planned):** Dangling nodes (single terminal connection) and floating nets (no DC path to ground). Severity warning; configurable. See [Dialect reference and net semantics](../8_dialect-reference-and-semantics.md).
- **Navigation (Go to Definition & Find References):**
    - Resolve references for subcircuits (`.subckt`) and models (`.model`), including through `.include` / `.lib`.
    - On `.lib 'file' entry` / `.include` lines, jump from the path to the file and from a `.lib` entry name to the section header.
    - Map parameter definitions (`.param`) to their usages in expressions.
- **Autocomplete and Snippets (planned):**
    - Offer context-aware suggestions for basic elements (R, C, L, diodes, transistors).
    - Provide templates for simulation directives (e.g., `.tran`, `.ac`, `.dc`, `.temp`).
- **Hover Documentation:**
    - **File-local:** Subcircuit pin order, in-file model parameters.
    - **Dialect reference:** Curated documentation for directives (`.tran`, `.ac`), `.option` keywords, element types, and common expressions — **authored per dialect** in a `reference/` corpus the LSP loads at runtime, not hard-coded in server logic. Coverage grows over time as you add entries for Ngspice, LTspice, and HSPICE.
- **Document Outline (Symbols):**
    - Index hierarchical structures, isolating `.subckt` blocks, `.model` definitions, and control blocks.

### 2.2 Formatter Capabilities

The formatting engine processes netlist files to enforce consistent layouts:

- **Columnar Alignment:** Align component names, nodes, model references, values, and parameters in tabular columns.
- **Case Normalization:** Enforce uppercase, lowercase, or preserve for leading `.directive` keywords.
- **Continuation-Line Standardization:** Fold and re-wrap multi-line statements with the `+` character using predictable indentation.
- **Comment Preservation:** Keep line-start comments (`*`, `;`, `$`); normalize spacing before inline `;` comments.

---

## 3. System Requirements

### 3.1 Functional Requirements

- **Dialect Support:** The parser must support standard SPICE variants, specifically LTspice, Ngspice, and HSPICE syntax.
- **Performance:** Code diagnostics must execute in under 100ms on files up to 50,000 lines.
- **Robustness:** The parser must gracefully recover from syntax errors to continue indexing subsequent parts of the file.

### 3.2 Technical & Architectural Requirements

- **Parser Technology:** Implement the parser using a formal grammar parser-generator like Tree-sitter. This ensures incremental parsing capability for low-latency editing.
- **Communication Protocol:** Conform strictly to the official LSP specification (JSON-RPC 2.0).
- **Distribution:** Package the LSP as a standalone executable (compiled Rust) with no external runtimes required.

---

## 4. Development plans

Capabilities that are not yet shipped, in rough priority:

| Focus | Notes |
|-------|-------|
| Completion | Element/directive suggestions; reuse reference corpus for docs |
| Connectivity | Dangling-node and floating-net diagnostics — [Dialect reference and net semantics](../8_dialect-reference-and-semantics.md) |
| Large-file / extracted mode | Size-gated defs-only analysis for 100+ MB netlists — [Large-file / extracted mode](3_large-file-extracted-mode.md) |
| Deeper dialect grammar | LTspice / HSPICE parse quirks beyond the shared grammar |

Shipped behavior is documented in [Architecture](../4_architecture.md) and [LSP features](../5_lsp-features.md). Multi-dialect selection and corpus authoring: [Multi-dialect support](2_multi-dialect-design.md).

### Demo and test strategy

| Layer | Method |
|-------|--------|
| Grammar | Tree-sitter corpus + Rust fixture tests |
| Parser | Golden diagnostics on `test-data/invalid/*` |
| LSP | JSON-RPC harness over stdio (subprocess or mock client) |
| VS Code | Extension Development Host (F5), Problems panel |
| CI | `pixi install && pixi run test` on every push |

Manual smoke: open `test-data/invalid/unclosed-subckt.cir`, fix `.ends`, confirm diagnostic clears. See [Demo and testing](../development/2_demo-and-test.md).

### VS Code as primary client

Distribution path:

1. **Development:** `spiceLsp.serverPath` points at `target/debug/spice-lsp`
2. **Side-load:** `.vsix` built with `vsce package`
3. **General availability:** Marketplace publish with platform-specific bundled binaries

Extension architecture (thin Node client, Rust server): [VS Code integration](../development/3_vscode-integration.md).

---

## 5. System Architecture & Implementation

### 5.1 Implementation Language

The LSP and formatter are implemented in **Rust** to satisfy the low-latency and performance requirements (<100ms on 50k lines) while guaranteeing thread safety and memory efficiency without a garbage collector.

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
│    │   Syntax, symbols, include graph; connectivity    │
│    │   planned                                         │
│    │                                                   │
│    ├─► [Reference Index] ◄── reference/<dialect>/     │
│    │   Hover docs for directives, options, elements    │
│    │                                                   │
│    └─► [Formatter Engine] ◄─────────────────────────┘   │
│        Columnar alignment, continuations, keyword case │
└────────────────────────────────────────────────────────┘
```

- **LSP Layer:** Handles connection lifecycle, text document synchronization, and capability routing.
- **Incremental Parsing:** Tree-sitter maintains an active syntax tree; edits re-parse only changed ranges.
- **Reference Index:** Loads structured JSON entries from `reference/` per active dialect; powers `textDocument/hover` and will enrich completion documentation. Maintained manually over time — see [Dialect reference and net semantics](../8_dialect-reference-and-semantics.md).
- **Net Graph (planned):** Builds terminal connectivity from instance lines; emits dangling-node and floating-net warnings.
- **Formatting Pipeline:** Line/token pretty-print → column rules → `TextEdit` (full document).

### 5.3 Key Dependencies

- **`tower-lsp`**: High-level LSP implementation framework for Rust built on Tokio.
- **`tree-sitter`**: Rust bindings to the incremental parsing library.
- **`tree-sitter-spice`**: Custom grammar for parsing SPICE dialects.
- **`serde` / `serde_json`**: Serialization and deserialization of LSP messages.
- **`clap`**: Robust command-line argument parser for standalone formatter CLI execution.
