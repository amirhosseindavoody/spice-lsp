# Introduction

**Last verified against:** LSP CLI accepts `--stdio` (vscode-languageclient); size-gated extracted analysis mode (`spiceLsp.analysisMode` / `extractedByteThreshold`); document formatting (`textDocument/formatting` + `spice-lsp format`); go-to-definition on `.lib` / `.include`; include/lib resolution; multi-dialect hover (default HSPICE); completion and connectivity still planned

spice-lsp is a language server and VS Code extension for SPICE circuit netlists. It gives you editor feedback while you write `.cir`, `.sp`, `.spf`, `.inc`, `.lib`, and related files — without running a simulator.

Use it when you want syntax and semantic checks, navigation, and dialect-aware help in the same place you edit the netlist.

## What you get

| Capability | What it does |
|------------|--------------|
| **Diagnostics** | Syntax errors plus warnings for duplicate names, unknown models/subcircuits, missing includes, and bad `.lib` sections |
| **Outline** | Document symbols for subcircuits, models, parameters, and instances |
| **Go to definition / find references** | Jump between `.subckt` / `.model` / `.param` definitions and their uses; also jump from `.include` / `.lib` paths (and `.lib` entry names) into the target file or section |
| **Include and library resolution** | Follow `.include` / `.inc` and HSPICE `.lib 'file' entry` so models and subcircuits in other files participate in checks and navigation |
| **Hover** | Dialect reference docs (HSPICE by default) plus file-local detail for subcircuit pins and in-file models |
| **Dialect selection** | Choose HSPICE, Ngspice, or LTspice (`spiceLsp.dialect`) so hover and related behavior match your simulator |
| **Formatting** | Columnar instance alignment, `+` wrap, directive keyword casing via Format Document or `spice-lsp format` |

The VS Code extension starts the `spice-lsp` binary over stdio. The same binary works with any LSP-capable editor.

## Install and try it

**VS Code:** install [SPICE Language Support](https://marketplace.visualstudio.com/items?itemName=AmirhosseinDavoody.spice-lsp) from the Marketplace, open a netlist, and edit — diagnostics and navigation should appear without extra setup.

**From source:** see [Getting Started](2_getting-started.md) for the pixi workflow (`pixi install`, `pixi run build`, `pixi run spice-lsp`).

Configure search paths for shared model libraries with `spiceLsp.libraryPaths`. Details: [Include and library resolution](9_include-and-lib-resolution.md).

## Why spice-lsp

SPICE netlists are text, but most editors treat them as plain files. spice-lsp is built so that:

- Feedback stays in the editor — no simulator round-trip for common mistakes
- The core is editor-agnostic (LSP); VS Code is the first client
- Dialect differences are first-class — hover and settings follow HSPICE, Ngspice, or LTspice
- Include and `.lib` chains are part of analysis, matching how real decks are structured

Goals and non-goals: [Principles](3_principles.md). Known gaps: [Limitations](7_limitations.md).

## Documentation map

| If you want… | Read |
|--------------|------|
| Setup, first run, editor install | [Getting Started](2_getting-started.md) |
| What the server implements | [LSP Features](5_lsp-features.md) |
| How crates and the pipeline fit together | [Architecture](4_architecture.md) |
| `.include` / `.lib` behavior | [Include and Library Resolution](9_include-and-lib-resolution.md) |
| Dialect docs and net semantics | [Dialect Reference and Net Semantics](8_dialect-reference-and-semantics.md) |
| Per-dialect reference pages | [Dialect reference catalog](reference/README.md) |
| Formatting rules and CLI | [Formatter](6_formatter.md) |
| Building from source / CI | [Build](development/1_build.md) |
| Extension layout and publishing | [VS Code Integration](development/3_vscode-integration.md) |

Repository quick start: [README.md](../README.md).
