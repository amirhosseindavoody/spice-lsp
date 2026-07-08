# Build

Pixi-managed build environment, tasks, CI, and release workflow.

## Environment

All tooling flows through pixi:

```bash
pixi install          # sync environment from pixi.lock
pixi run <task>       # run a defined task
pixi run cargo build  # ad-hoc command in the env
```

Add dependencies with the CLI (do not hand-edit versions):

```bash
pixi add rust=1.96
pixi add nodejs=22       # when VS Code extension work starts
pixi add mdbook          # already in pixi.toml
pixi add --pypi pytest   # example PyPI package
```

## Planned pixi tasks

Add these to `[tasks]` in `pixi.toml` as the workspace grows:

| Task | Command | Purpose |
|------|---------|---------|
| `build` | `cargo build --release` | Production binary |
| `build-dev` | `cargo build` | Fast debug builds |
| `test` | `cargo test --workspace` | All unit + integration tests |
| `test-parser` | `cargo test -p spice-parser` | Parser fixtures only |
| `test-lsp` | `cargo test -p spice-lsp` | LSP integration tests |
| `spice-lsp` | `cargo run -p spice-lsp` | Run language server (stdio) |
| `fmt` | `cargo fmt --all` | Rust formatting |
| `clippy` | `cargo clippy --workspace -- -D warnings` | Lint |
| `mdbook-build` | `mdbook build docs` | Static doc site |
| `mdbook-serve` | `mdbook serve docs -n 127.0.0.1 -p 3000` | Live doc preview |
| `ext-install` | `npm install` in `editors/vscode` | Extension deps |
| `ext-compile` | `npm run compile` | Build extension TS |
| `ext-package` | `vsce package` | `.vsix` for side-loading |

Example `pixi.toml` fragment:

```toml
[tasks]
build = "cargo build --release"
test = "cargo test --workspace"
spice-lsp = "cargo run -p spice-lsp"
mdbook-serve = "mdbook serve docs -n 127.0.0.1 -p 3000"
```

## Rust workspace layout

Root `Cargo.toml`:

```toml
[workspace]
members = ["crates/spice-parser", "crates/spice-lsp"]
resolver = "2"
```

Release profile (recommended once benchmarks exist):

```toml
[profile.release]
lto = true
codegen-units = 1
```

## Building the Tree-sitter grammar

Grammar crate under `tree-sitter-spice/` is built via `build.rs` in `spice-parser`:

```bash
pixi run cargo build -p spice-parser
```

After grammar edits, run Tree-sitter's test harness (once added):

```bash
pixi run cargo test -p tree-sitter-spice
# or: tree-sitter test (if CLI added via pixi)
```

## CI (recommended)

GitHub Actions workflow stages:

1. `pixi install`
2. `pixi run fmt -- --check`
3. `pixi run clippy`
4. `pixi run test`
5. `pixi run mdbook-build` (optional, docs PRs)

Cache `~/.pixi` and `target/` between runs.

## Release builds

Ship a single static binary per platform:

```bash
pixi run cargo build --release -p spice-lsp
# artifact: target/release/spice-lsp
```

Cross-compile with `cross` or platform matrix in CI. Attach binaries to GitHub Releases; the VS Code extension downloads or bundles the matching platform binary (see [VS Code integration](4_vscode-integration.md)).

## Documentation site

## Documentation site

Build locally:

```bash
pixi run mdbook-build
# output: docs/book/
```

Preview:

```bash
pixi run mdbook-serve
```

### CI and GitHub Pages

The [Deploy docs](../../.github/workflows/deploy-docs.yml) workflow runs on pushes to `main` when `docs/`, `pixi.toml`, or `pixi.lock` change. It:

1. Runs `pixi run mdbook-build`
2. Pushes the output to the `gh-pages` branch
3. Configures GitHub Pages (source: `gh-pages` / root) and sets the repository **Website** field

Published URL: **https://amirhosseindavoody.github.io/spice-lsp/**

Trigger a manual deploy from the Actions tab via **workflow_dispatch** if needed.

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `cargo: command not found` | Run `pixi install`; use `pixi run cargo` |
| Tree-sitter `build.rs` fails | Ensure C compiler available in pixi env (`pixi add gcc` on Linux) |
| Extension can't find binary | Set `spiceLsp.serverPath` in VS Code settings to absolute path |
| LSP hangs on start | Normal for stdio servers waiting for JSON-RPC input |

## Related

- [Getting started](../2_getting-started.md)
- [MVP guide](2_mvp.md)
- [Demo and testing](3_demo-and-test.md)
