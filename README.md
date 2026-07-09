# spice-lsp

Language server and formatter for [SPICE](https://en.wikipedia.org/wiki/SPICE) circuit simulation netlists.

**Current status:** v0.2 — syntax + semantic diagnostics, document outline, go to definition, find references; VS Code extension with bundled binaries; Marketplace publish on each push to `main`.

## VS Code extension

Install from the Marketplace (search **SPICE Language Support**) once published, or side-load a `.vsix`:

```bash
pixi run build
pixi run ext-package
code --install-extension editors/vscode/spice-lsp-0.2.0.vsix
```

Marketplace publish is automated via GitHub Actions — see [VS Code integration](docs/development/4_vscode-integration.md#publishing).

## End goal

Ship a **VS Code extension** that starts the `spice-lsp` binary over stdio and provides:

- Real-time syntax diagnostics as you edit netlists (MVP)
- Navigation, completion, and formatting (v0.2–v0.4)
- **Dialect-aware documentation on hover** — curated reference files you maintain per Ngspice / LTspice / HSPICE (v0.5)
- **Connectivity warnings** — dangling nodes and floating nets highlighted before simulation (v0.5)

See [VS Code integration](docs/development/4_vscode-integration.md) and [Dialect reference and net semantics](docs/8_dialect-reference-and-semantics.md).

## Prerequisites

- [pixi](https://pixi.sh/latest/#installation)

## Quick start

```bash
git clone https://github.com/amirhosseindavoody/spice-lsp.git
cd spice-lsp
pixi install
```

Once the Rust workspace is added (MVP step 1):

```bash
pixi run build          # compile the LSP binary
pixi run test           # unit and integration tests
pixi run spice-lsp      # run the language server (stdio)
```

Build the documentation site locally:

```bash
pixi run mdbook-build
# output: docs/book/
```

Preview with live reload:

```bash
pixi run mdbook-serve
```

Published site (after deploy workflow + one-time Pages setup): **https://amirhosseindavoody.github.io/spice-lsp/**

First-time setup (repository admin, after the first deploy):

```bash
./scripts/setup-github-pages.sh
```

## MVP in one page

The fastest path to something you can **demo in VS Code** is intentionally narrow:

| MVP delivers | MVP defers |
|--------------|------------|
| Stdio LSP server (`initialize`, text sync, `publishDiagnostics`) | Dialect reference hover (curated `reference/` corpus) |
| Tree-sitter parse of a single dialect (start with Ngspice) | Floating-net / dangling-node analysis |
| Syntax diagnostics from the parse tree | Formatter, completion, navigation |
| VS Code extension that launches the binary and shows squiggles | Multi-dialect reference libraries |

**Build order:** Cargo workspace → minimal grammar → LSP skeleton → sample netlist fixtures → VS Code extension → integration test that speaks JSON-RPC.

Full step-by-step instructions: [MVP guide](docs/development/2_mvp.md).

## Demo and test

| What you want to verify | How |
|-------------------------|-----|
| Parser on sample netlists | `pixi run test` (fixture tests in `crates/spice-parser`) |
| LSP handshake and diagnostics | `pixi run test --package spice-lsp` (stdio integration test) |
| Manual smoke test | Open a `.cir` file, run **SPICE LSP: Restart Server**, introduce a syntax error |
| Extension in isolation | `cd editors/vscode && npm run compile && F5` (Extension Development Host) |

Details: [Demo and testing](docs/development/3_demo-and-test.md).

## Documentation

The book lives under `docs/` and is built with [mdBook](https://rust-lang.github.io/mdBook/).

| Chapter | Topic |
|---------|-------|
| [Getting started](docs/2_getting-started.md) | Setup, pixi workflow, first run |
| [Architecture](docs/4_architecture.md) | Crates, phased rollout (MVP → v0.5) |
| [Dialect reference and net semantics](docs/8_dialect-reference-and-semantics.md) | Curated docs + floating/dangling analysis (post-MVP) |
| [MVP guide](docs/development/2_mvp.md) | Minimal implementation before more features |
| [VS Code integration](docs/development/4_vscode-integration.md) | Extension structure and publishing |
| [Design (internal)](docs/internal/1_design.md) | Requirements and capability spec |

Navigation: [docs/SUMMARY.md](docs/SUMMARY.md).

## Project layout (target)

```
spice-lsp/
├── Cargo.toml                 # Rust workspace root (MVP)
├── crates/
│   ├── spice-parser/          # Tree-sitter grammar + diagnostics
│   ├── spice-reference/       # Dialect doc index (v0.5)
│   └── spice-lsp/             # tower-lsp binary
├── reference/                 # Curated dialect docs — authored over time (v0.5)
│   ├── ngspice/
│   ├── ltspice/
│   └── hspice/
├── tree-sitter-spice/         # Grammar sources and queries
├── editors/
│   └── vscode/                # VS Code extension (MVP client)
├── test-data/                 # Sample netlists for tests and demos
├── docs/                      # mdBook source
├── pixi.toml                  # Tasks and dependencies
└── README.md
```

## Development

All dependencies and the build environment are managed by pixi:

```bash
pixi add <package>              # conda dependency
pixi add --pypi <package>       # PyPI dependency
pixi run <task>                 # run a defined task
```

Do not hand-edit dependency versions in `pixi.toml`; use the pixi CLI.

See [Build](docs/development/1_build.md) for pixi tasks, CI, and release builds.
