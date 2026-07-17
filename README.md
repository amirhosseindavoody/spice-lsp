# spice-lsp

Language server and formatter for [SPICE](https://en.wikipedia.org/wiki/SPICE) circuit simulation netlists.

**Current status:** document formatting (LSP + `spice-lsp format`), include/lib resolution for `.model` / `.subckt` via `.include` and `.lib`, multi-dialect hover (default HSPICE), syntax + semantic diagnostics, outline, go to definition, find references; VS Code extension with bundled binaries.

## VS Code extension

Install from the [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=AmirhosseinDavoody.spice-lsp) (**SPICE Language Support**), or side-load a `.vsix`:

```bash
pixi run build
pixi run ext-package
code --install-extension editors/vscode/spice-lsp-0.2.0.vsix
```

Marketplace publish is automated via GitHub Actions — see [VS Code integration](docs/development/3_vscode-integration.md#publishing).

## End goal

Ship a **VS Code extension** that starts the `spice-lsp` binary over stdio and provides:

- Real-time syntax + semantic diagnostics as you edit netlists (**shipped**)
- Document outline, go to definition, and find references (**shipped**)
- Document formatting — columnar alignment and `+` continuations (**shipped**)
- Completion (planned)
- **Dialect-aware documentation on hover** — curated reference files you maintain per Ngspice / LTspice / HSPICE (**shipped**)
- **Connectivity warnings** — dangling nodes and floating nets highlighted before simulation (planned)

See [VS Code integration](docs/development/3_vscode-integration.md) and [Dialect reference and net semantics](docs/8_dialect-reference-and-semantics.md).

## Prerequisites

- [pixi](https://pixi.sh/latest/#installation)

## Quick start

```bash
git clone https://github.com/amirhosseindavoody/spice-lsp.git
cd spice-lsp
pixi install
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

## What v0.3 delivers

| Shipped | Still deferred |
|---------|----------------|
| Stdio LSP server (`initialize`, text sync, `publishDiagnostics`) | Floating-net / dangling-node analysis |
| Tree-sitter parse (shared grammar; dialect profile) | Completion |
| Syntax + semantic diagnostics | Deep LTspice / HSPICE grammar splits |
| Document outline, go to definition, find references | Workspace-wide symbol search |
| Document formatting (LSP + `spice-lsp format` CLI) | Dialect-specific formatter profiles |
| `.include` / `.lib` model & subckt resolution | Cross-file find-references / net graph |
| Dialect setting (default HSPICE) + curated hover corpus | Per-file dialect overrides |
| VS Code extension (Marketplace, bundled binaries, highlighting) | Windows arm64 bundled binary |

**Build order (historical):** Cargo workspace → minimal grammar → LSP skeleton → sample netlist fixtures → VS Code extension → integration test that speaks JSON-RPC.

## Demo and test

| What you want to verify | How |
|-------------------------|-----|
| Parser on sample netlists | `pixi run test` (fixture tests in `crates/spice-parser`) |
| Formatter goldens | `pixi run cargo test -p spice-parser --test format_golden` |
| Format a netlist | `pixi run format-spice -- --write path/to/file.cir` |
| LSP handshake and diagnostics | `pixi run test --package spice-lsp` (stdio integration test) |
| Manual smoke test | Open a `.cir` file, run **SPICE LSP: Restart Server**, introduce a syntax error |
| Extension in isolation | `cd editors/vscode && npm run compile && F5` (Extension Development Host) |

Details: [Demo and testing](docs/development/2_demo-and-test.md).

## Documentation

The book lives under `docs/` and is built with [mdBook](https://rust-lang.github.io/mdBook/).

| Chapter | Topic |
|---------|-------|
| [Getting started](docs/2_getting-started.md) | Setup, pixi workflow, first run |
| [Architecture](docs/4_architecture.md) | Crates, phased rollout |
| [Dialect reference and net semantics](docs/8_dialect-reference-and-semantics.md) | Curated docs + floating/dangling analysis |
| [Include and library resolution](docs/9_include-and-lib-resolution.md) | `.include` / `.lib` model and subckt resolution |
| [Dialect reference catalog](docs/reference/README.md) | Generated mdBook pages from `reference/` JSON |
| [VS Code integration](docs/development/3_vscode-integration.md) | Extension structure and publishing |
| [Design (internal)](docs/internal/1_design.md) | Requirements and capability spec |

Navigation: [docs/SUMMARY.md](docs/SUMMARY.md).

## Project layout

```
spice-lsp/
├── Cargo.toml                 # Rust workspace root
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
│   └── vscode/                # VS Code extension
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

## License

MIT — see [LICENSE](LICENSE).
