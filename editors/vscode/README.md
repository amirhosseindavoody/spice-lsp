# SPICE Language Support

Language support for [SPICE](https://en.wikipedia.org/wiki/SPICE) circuit simulation netlists, powered by the [spice-lsp](https://github.com/amirhosseindavoody/spice-lsp) language server.

## Quick start

1. **Install** this extension from the Marketplace (no separate language-server install).
2. **Open a netlist** — any `.cir`, `.sp`, `.spf`, `.net`, `.ckt`, `.inc`, or `.lib` file. VS Code should show the language mode **SPICE** in the status bar.
3. **Watch the Problems panel** (`View → Problems`) for syntax errors and semantic warnings as you type — for example an unclosed `.subckt`, a duplicate instance name, or a reference to an undefined model.
4. **Use the outline** (`View → Outline`, or Explorer → Outline) to jump between `.subckt`, `.model`, `.param`, and instances.
5. **Navigate** with **Go to Definition** (`F12`) on a subcircuit or model name — including names defined in `.include` / `.lib` files — and **Find All References** (`Shift+F12`) to list every use in the current buffer.
6. **Optional:** run **SPICE LSP: Create Demo Folder** from the Command Palette to drop a `spice-lsp-demo/` folder (with `.sp` samples) into your workspace so you can try same-file and cross-file navigation immediately.

That is enough for day-to-day editing. The matching `spice-lsp` binary for your platform is bundled; leave `spiceLsp.serverPath` empty unless you are developing the server yourself.

If nothing happens after opening a file, check **Output → SPICE Language Server**, then run **SPICE LSP: Restart Server** from the Command Palette. That Output channel is created as soon as the extension activates.

To switch dialect, use **SPICE LSP: Set Dialect…** (or the status-bar dialect chip). Requires extension **0.2.10+**. If VS Code says the command is not registered, update/reload the extension.

To scaffold sample netlists, run **SPICE LSP: Create Demo Folder**. It copies HSPICE `.sp` / `.lib` samples into `spice-lsp-demo/` under your opened workspace (`same-file.sp` for in-file jumps; `top.sp` + `models.sp` for `.include`; `top-lib.sp` + `corners.lib` for HSPICE `.lib`) and sets the dialect to HSPICE.

## Features

- **Syntax highlighting** — comments (`*` / `;` / `$`), directives, instances, and numbers
- **Syntax diagnostics** — unclosed `.subckt` blocks, parse errors, and related issues
- **Semantic warnings** — duplicate component names, undefined model/subcircuit references
- **Document outline** — hierarchical view of subcircuits, models, parameters, and instances
- **Go to definition** — jump from subcircuit/model references to `.subckt` / `.model` definitions, including through `.include` and HSPICE `.lib` sections
- **Find references** — list all usages of a subcircuit, model, or parameter in the open buffer
- **Dialect-aware hover** — documentation for directives and elements from the active dialect (default **HSPICE**)

Supported file extensions: `.cir`, `.sp`, `.spf`, `.net`, `.ckt`, `.inc`, `.lib`. Toggle Comment uses `*` (VS Code allows one line-comment marker); `;` and `$` still highlight as comments.

## Bundled platforms

The extension includes a prebuilt `spice-lsp` binary for:

| Platform | Arch |
|----------|------|
| Linux | x64, arm64 (glibc **2.31+**, e.g. Ubuntu 20.04 / Debian 11) |
| macOS | Intel (x64), Apple Silicon (arm64) |
| Windows | x64 |

Other platforms (or older Linux glibc) need a `spice-lsp` binary on your `PATH`, or set `spiceLsp.serverPath` to an absolute path and run **SPICE LSP: Restart Server**.

## Settings

| Setting | Description |
|---------|-------------|
| `spiceLsp.dialect` | Active dialect: `hspice` (default), `ngspice`, or `ltspice` |
| `spiceLsp.libraryPaths` | Extra directories for resolving `.include` / `.lib` paths |
| `spiceLsp.include.maxDepth` | Max nesting depth for include/lib chains (default `16`) |
| `spiceLsp.serverPath` | Override the bundled language-server binary with a custom path |
| `spiceLsp.trace.server` | Trace LSP communication: `off`, `messages`, or `verbose` |

Use **SPICE LSP: Set Dialect…** (or the status-bar dialect chip) to switch. Hover tips come from the curated `reference/` corpus for the active dialect.

## Documentation

Project docs: [spice-lsp book](https://amirhosseindavoody.github.io/spice-lsp/)

## License

MIT — see the [repository](https://github.com/amirhosseindavoody/spice-lsp) for details.
