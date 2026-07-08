# spice-lsp

Language server for [SPICE](https://en.wikipedia.org/wiki/SPICE) circuit simulation netlists. The project is in early scaffolding: the Rust toolchain and build environment are set up with [pixi](https://pixi.sh); the LSP implementation has not been added yet.

## Prerequisites

- [pixi](https://pixi.sh/latest/#installation)

## Quick start

Clone the repository, install the environment, and run commands inside pixi:

```bash
git clone https://github.com/amirhosseindavoody/spice-lsp.git
cd spice-lsp
pixi install
```

Once a Rust crate is added, build and test through pixi:

```bash
pixi run cargo build
pixi run cargo test
```

## Development

All dependencies and the build environment are managed by pixi. Use pixi CLI commands to add or remove packages instead of editing dependency sections in `pixi.toml` by hand:

```bash
pixi add <package>
pixi add --pypi <package>
pixi remove <package>
```

Run one-off commands in the project environment with `pixi run …`.

## Project layout

| Path | Purpose |
|------|---------|
| `pixi.toml` | Workspace manifest (channels, platforms, tasks, dependencies) |
| `pixi.lock` | Locked dependency versions |
| `.cursor/rules/` | Cursor agent rules for this repo |
| `docs/` | Project documentation (to be expanded) |

## Documentation

Docs live under `docs/` for [mdBook](https://rust-lang.github.io/mdBook/). Chapter files use numbered names (`2_getting-started.md`, …); navigation is defined in [docs/SUMMARY.md](docs/SUMMARY.md). The book landing page is [docs/README.md](docs/README.md).
