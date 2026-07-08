# Introduction

**Last verified against:** documentation expansion (pre-MVP scaffolding — no LSP crate yet)

spice-lsp is a language server and formatter for SPICE circuit simulation netlists. The **end goal** is a VS Code extension that provides real-time diagnostics, navigation, and formatting while editing `.cir`, `.sp`, and related netlist files.

This book is generated with [mdBook](https://rust-lang.github.io/mdBook/) from the `docs/` directory.

## Where to start

| If you want to… | Read |
|-----------------|------|
| Set up the repo and run builds | [Getting Started](2_getting-started.md) |
| Understand MVP scope and ship a demo fast | [MVP Guide](development/2_mvp.md) |
| Demo or test manually and in CI | [Demo and Testing](development/3_demo-and-test.md) |
| Wire up VS Code | [VS Code Integration](development/4_vscode-integration.md) |
| Understand crate layout and phases | [Architecture](4_architecture.md) |

Quick setup (pixi, clone, build) also lives in the repository [README.md](../README.md).

## Roadmap at a glance

```
MVP          → syntax diagnostics in VS Code
v0.2         → outline, go to definition, references
v0.3         → completion, hover, snippets
v0.4         → formatter, dialect settings
```

## Build the book locally

Once `mdbook` is added to the pixi environment:

```bash
pixi run mdbook serve docs
```

Open the URL printed in the terminal (default `http://localhost:3000`).
