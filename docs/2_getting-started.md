# Getting Started

This chapter covers environment setup and the shortest path from clone to a running language server.

## Prerequisites

| Tool | Purpose |
|------|---------|
| [pixi](https://pixi.sh/latest/#installation) | Manages Rust, Node (for the VS Code extension), and build tasks |
| Git | Clone and contribute |

You do **not** need a system-wide Rust install. Pixi provides the toolchain pinned in `pixi.toml`.

## Clone and install

```bash
git clone https://github.com/amirhosseindavoody/spice-lsp.git
cd spice-lsp
pixi install
```

`pixi install` creates a reproducible environment with the Rust compiler and (once added) Node.js, mdBook, and other dev tools.

## Verify the environment

```bash
pixi run rustc --version
pixi run cargo --version
```

Both commands should succeed and report Rust ≥ 1.96.

## Build (after MVP scaffolding lands)

Once the Cargo workspace exists, build and test through pixi tasks:

```bash
pixi run build
pixi run test
```

Run the language server directly (it communicates over stdio — it will appear to hang; that is normal):

```bash
pixi run spice-lsp
```

Press Ctrl+C to stop.

## Open sample netlists

Create or copy a minimal netlist for manual testing:

```spice
* demo.cir — Ngspice-style
.title Simple RC
R1 in out 1k
C1 out 0 1u
V1 in 0 DC 1
.tran 1u 1m
.end
```

Save as `demo.cir` in the repo root or under `test-data/`.

## Editor integration (MVP)

### VS Code (primary target)

1. Build the LSP binary: `pixi run build`
2. Open the extension folder: `editors/vscode`
3. Install JS dependencies: `npm install`
4. Press **F5** to launch an Extension Development Host with the SPICE extension loaded
5. Open `demo.cir` and confirm diagnostics appear

Full extension setup: [VS Code integration](development/4_vscode-integration.md).

### Other editors

Any editor with generic LSP client support can point at the `spice-lsp` binary:

| Editor | Configuration |
|--------|---------------|
| Neovim | `lspconfig` custom server block with `cmd = { "spice-lsp" }` |
| Helix | `[language-server.spice-lsp]` in `languages.toml` |
| Zed | Extension or `lsp` settings (once published) |

File extensions to associate: `.cir`, `.sp`, `.net`, `.ckt` (dialect-dependent).

## Recommended first contribution path

If you are new to the repo, follow this order:

1. Read [Principles](3_principles.md) — know what is in and out of scope for MVP
2. Follow [MVP guide](development/2_mvp.md) — implement or review each milestone
3. Use [Demo and testing](development/3_demo-and-test.md) — verify each layer before adding features
4. Read [Architecture](4_architecture.md) — understand where new code belongs

## Next steps

- [MVP guide](development/2_mvp.md) — build the smallest demoable server
- [Architecture](4_architecture.md) — crate layout and data flow
- [Build](development/1_build.md) — pixi tasks and CI
