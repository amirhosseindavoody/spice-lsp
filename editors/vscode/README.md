# SPICE Language Support

VS Code extension for [spice-lsp](https://github.com/amirhosseindavoody/spice-lsp).

## Development

1. Build the language server from the repo root:

   ```bash
   pixi run build-dev
   ```

2. Install and compile the extension:

   ```bash
   npm install
   npm run compile
   ```

3. Press **F5** to open the Extension Development Host.

4. Open a `.cir` file such as `test-data/invalid/unclosed-subckt.cir`.

## Settings

| Setting | Description |
|---------|-------------|
| `spiceLsp.serverPath` | Path to the `spice-lsp` binary |
| `spiceLsp.trace.server` | `off`, `messages`, or `verbose` |

If `serverPath` is empty, the extension tries `../../target/debug/spice-lsp` relative to the compiled extension (works when launched from this repo).
