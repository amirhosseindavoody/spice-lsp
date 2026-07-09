# Introduction

**Last verified against:** v0.3 — multi-dialect setting (default HSPICE), expanded HSPICE reference corpus (`.data` / `.dc` / `.op` + common controls), dialect-aware hover, generated mdBook dialect catalog; VS Code commands register before LSP start; Linux glibc 2.31+ binaries; Marketplace publish on each push to `main`

spice-lsp is a language server and formatter for SPICE circuit simulation netlists. The **end goal** is a VS Code extension that provides real-time diagnostics, dialect-aware documentation on hover, navigation, formatting, and connectivity warnings while editing `.cir`, `.sp`, `.spf`, and related files.

This book is generated with [mdBook](https://rust-lang.github.io/mdBook/) from the `docs/` directory.

## How to read this book

| Stage | Chapters |
|-------|----------|
| **Setup and ship MVP** | [Getting Started](2_getting-started.md) → [Principles](3_principles.md) → [MVP Guide](development/2_mvp.md) → [Demo and Testing](development/3_demo-and-test.md) |
| **Understand the system** | [Architecture](4_architecture.md) → [LSP Features](5_lsp-features.md) |
| **Long-term direction** | [Dialect Reference and Net Semantics](8_dialect-reference-and-semantics.md) → [Dialect reference catalog](reference/README.md) → [Formatter](6_formatter.md) → [Limitations](7_limitations.md) |
| **VS Code** | [VS Code Integration](development/4_vscode-integration.md) |

Quick setup lives in the repository [README.md](../README.md).

## Roadmap at a glance

```
MVP     Syntax diagnostics in VS Code
v0.2    Outline, go to definition, duplicate/undefined warnings
v0.3    Completion, file-local hover
v0.4    Formatter, dialect setting
v0.5    Curated dialect reference (hover) + dangling-node / floating-net warnings
```

v0.5 is where you maintain `reference/<dialect>/` documentation and the LSP begins SPICE-specific semantic assistance beyond syntax. Not part of MVP.

## Build the book locally

```bash
pixi run mdbook-build    # render static site to docs/book/
pixi run mdbook-serve      # preview at http://127.0.0.1:3000
```

## Published site

Pushes to `main` that touch `docs/` deploy the book to the `gh-pages` branch via [`.github/workflows/deploy-docs.yml`](../.github/workflows/deploy-docs.yml).

**https://amirhosseindavoody.github.io/spice-lsp/**
