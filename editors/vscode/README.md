# SPICE Language Support

Language support for [SPICE](https://en.wikipedia.org/wiki/SPICE) circuit simulation netlists, powered by the [spice-lsp](https://github.com/amirhosseindavoody/spice-lsp) language server.

## Features

- **Syntax diagnostics** — unclosed `.subckt` blocks, parse errors, and related issues
- **Semantic warnings** — duplicate component names, undefined model/subcircuit references
- **Document outline** — hierarchical view of subcircuits, models, parameters, and instances
- **Go to definition** — jump from subcircuit references to `.subckt` definitions
- **Find references** — list all usages of a subcircuit, model, or parameter

Supported file extensions: `.cir`, `.sp`, `.net`, `.ckt`.

## Getting started

Install the extension from the Marketplace — no extra setup is required. The matching `spice-lsp` binary for your platform (Linux, macOS, or Windows on x64/arm64) is bundled automatically.

Open a netlist file and diagnostics, outline, and navigation features activate when the file is recognized as SPICE.

## Settings

| Setting | Description |
|---------|-------------|
| `spiceLsp.serverPath` | Override the bundled language-server binary with a custom path |
| `spiceLsp.trace.server` | Trace LSP communication: `off`, `messages`, or `verbose` |

Use **SPICE LSP: Restart Server** from the Command Palette after changing `serverPath`.

## Documentation

Project docs: [spice-lsp book](https://amirhosseindavoody.github.io/spice-lsp/)

## License

MIT — see the [repository](https://github.com/amirhosseindavoody/spice-lsp) for details.
