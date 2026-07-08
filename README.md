# spice-lsp

Language server and formatter for [SPICE](https://en.wikipedia.org/wiki/SPICE) circuit simulation netlists.

**Current status:** early scaffolding. The Rust toolchain and [pixi](https://pixi.sh) build environment are in place; the LSP crate, parser, and VS Code extension are not implemented yet. The documentation describes the target architecture and a concrete path to a demoable MVP.

## End goal

Ship a **VS Code extension** that starts the `spice-lsp` binary over stdio and provides:

- Real-time syntax diagnostics as you edit `.cir`, `.sp`, and related netlist files
- Document outline for subcircuits and models (post-MVP)
- Go to definition, hover, completion, and format-on-save (later phases)

See [VS Code integration](docs/development/4_vscode-integration.md) for the extension layout and dev workflow.

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

Build the docs locally (after `mdbook` is added to the pixi environment):

```bash
pixi run mdbook serve docs
```

## MVP in one page

The fastest path to something you can **demo in VS Code** is intentionally narrow:

| MVP delivers | MVP defers |
|--------------|------------|
| Stdio LSP server (`initialize`, text sync, `publishDiagnostics`) | Semantic analysis (floating nodes, duplicate IDs) |
| Tree-sitter parse of a single dialect (start with Ngspice) | Formatter |
| Syntax diagnostics from the parse tree | Completion, hover, go-to-definition |
| VS Code extension that launches the binary and shows squiggles | HSPICE / LTspice dialect switches |

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
| [Architecture](docs/4_architecture.md) | Crates, parser/LSP pipeline, phased rollout |
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
│   └── spice-lsp/             # tower-lsp binary
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
