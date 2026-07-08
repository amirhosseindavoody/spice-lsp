# SPICE Netlist LSP and Formatter Design Document

## 1. Executive Summary

This document defines the system design, capabilities, and requirements for a Language Server Protocol (LSP) and Formatter tailored for SPICE (Simulation Program with Integrated Circuit Emphasis) netlists. The goal is to improve developer velocity, reduce syntax errors, and enforce stylistic consistency across analog and mixed-signal simulation workflows.

---

## 2. System Capabilities

### 2.1 Language Server Protocol (LSP) Capabilities

The LSP server implements the following capabilities to provide real-time IDE feedback:

- **Syntax and Semantic Diagnostics:**
    - Detect syntax violations (e.g., missing subcircuit terminators `.ends`, incorrect line continuations with `+`).
    - Flag semantic issues (e.g., floating nodes, duplicate component identifiers, missing model definitions referenced by active devices).
- **Navigation (Go to Definition & Find References):**
    - Resolve references for subcircuits (`.subckt`) and models (`.model`).
    - Map parameter definitions (`.param`) to their usages in expressions.
- **Autocomplete and Snippets:**
    - Offer context-aware suggestions for basic elements (R, C, L, diodes, transistors).
    - Provide templates for simulation directives (e.g., `.tran`, `.ac`, `.dc`, `.temp`).
- **Hover Documentation:**
    - Display terminal/pin order mappings for subcircuits and complex devices on hover.
    - Show parameter units and default values.
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

## 4. System Architecture & Implementation

### 4.1 Implementation Language

The LSP and formatter will be implemented in **Rust** to satisfy the low-latency and performance requirements (<100ms on 50k lines) while guaranteeing thread safety and memory efficiency without a garbage collector.

### 4.2 Architecture & Design

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
│    │   Traverses CST to find syntax/semantic issues    │
│    │                                                   │
│    └─► [Formatter Engine] ◄─────────────────────────┘   │
│        Applies columnar alignment & continuation formatting│
└────────────────────────────────────────────────────────┘
```

- **LSP Layer:** Handles connection lifecycle, text document synchronization (`textDocument/didOpen`, `textDocument/didChange`), and capability routing.
- **Incremental Parsing:** Tree-sitter maintains an active syntax tree in memory. On document edits, only modified ranges are re-parsed, keeping latency negligible.
- **Formatting Pipeline:** The formatter reads the CST, applies structural rules (such as aligning nodes and standardizing `+` continuation lines), and outputs a unified set of `TextEdit` actions.

### 4.3 Key Dependencies

- **`tower-lsp`**: High-level LSP implementation framework for Rust built on Tokio.
- **`tree-sitter`**: Rust bindings to the incremental parsing library.
- **`tree-sitter-spice`**: Custom or community grammar for parsing SPICE dialects.
- **`serde` / `serde_json`**: Serialization and deserialization of LSP messages.
- **`clap`**: Robust command-line argument parser for standalone formatter CLI execution.
