# SPICE Language Support

Language support for [SPICE](https://en.wikipedia.org/wiki/SPICE) circuit simulation netlists, powered by the [spice-lsp](https://github.com/amirhosseindavoody/spice-lsp) language server.

## Quick start

1. **Install** this extension from the Marketplace (no separate language-server install).
2. **Open a netlist** — any `.cir`, `.sp`, `.net`, or `.ckt` file. VS Code should show the language mode **SPICE** in the status bar.
3. **Watch the Problems panel** (`View → Problems`) for syntax errors and semantic warnings as you type — for example an unclosed `.subckt`, a duplicate instance name, or a reference to an undefined model.
4. **Use the outline** (`View → Outline`, or Explorer → Outline) to jump between `.subckt`, `.model`, `.param`, and instances.
5. **Navigate** with **Go to Definition** (`F12`) on a subcircuit or model name, and **Find All References** (`Shift+F12`) to list every use.

That is enough for day-to-day editing. The matching `spice-lsp` binary for your platform is bundled; leave `spiceLsp.serverPath` empty unless you are developing the server yourself.

If nothing happens after opening a file, check **Output → SPICE Language Server**, then run **SPICE LSP: Restart Server** from the Command Palette.

## Features

- **Syntax diagnostics** — unclosed `.subckt` blocks, parse errors, and related issues
- **Semantic warnings** — duplicate component names, undefined model/subcircuit references
- **Document outline** — hierarchical view of subcircuits, models, parameters, and instances
- **Go to definition** — jump from subcircuit references to `.subckt` definitions
- **Find references** — list all usages of a subcircuit, model, or parameter

Supported file extensions: `.cir`, `.sp`, `.net`, `.ckt`.

## Bundled platforms

The extension includes a prebuilt `spice-lsp` binary for:

| Platform | Arch |
|----------|------|
| Linux | x64, arm64 |
| macOS | Intel (x64), Apple Silicon (arm64) |
| Windows | x64 |

Other platforms need a `spice-lsp` binary on your `PATH`, or set `spiceLsp.serverPath` to an absolute path and run **SPICE LSP: Restart Server**.

## Settings

| Setting | Description |
|---------|-------------|
| `spiceLsp.serverPath` | Override the bundled language-server binary with a custom path |
| `spiceLsp.trace.server` | Trace LSP communication: `off`, `messages`, or `verbose` |

## Documentation

Project docs: [spice-lsp book](https://amirhosseindavoody.github.io/spice-lsp/)

## License

MIT — see the [repository](https://github.com/amirhosseindavoody/spice-lsp) for details.
