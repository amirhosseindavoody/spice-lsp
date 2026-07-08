# VS Code Integration

End goal for spice-lsp: a **VS Code extension** that launches the Rust language server and provides a first-class editing experience for SPICE netlists.

This chapter covers extension layout, development workflow, configuration, and publishing.

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│ VS Code Extension Host (Node.js)                           │
│                                                              │
│  package.json ── contributes languages, config, commands     │
│  extension.ts  ── activates LanguageClient                   │
│  language-configuration.json ── comments, brackets, auto-close│
└───────────────────────────┬──────────────────────────────────┘
                            │ spawns process
                            ▼
┌──────────────────────────────────────────────────────────────┐
│ spice-lsp binary (Rust, stdio JSON-RPC)                      │
│  initialize → didOpen/didChange → publishDiagnostics       │
└──────────────────────────────────────────────────────────────┘
```

The extension is intentionally thin: **no parsing in TypeScript**. All language intelligence stays in Rust so Neovim and other clients can share the same binary.

## Repository layout

```
editors/vscode/
├── .vscode/
│   ├── launch.json          # F5 Extension Development Host
│   └── tasks.json           # compile before launch
├── package.json
├── tsconfig.json
├── src/
│   └── extension.ts
├── language-configuration.json
└── README.md                # Marketplace-facing extension readme
```

## package.json

Key fields:

| Field | Purpose |
|-------|---------|
| `engines.vscode` | Minimum VS Code version |
| `activationEvents` | `onLanguage:spice` — lazy activate |
| `main` | `./out/extension.js` (compiled output) |
| `contributes.languages` | Register `spice` language id and file extensions |
| `contributes.configuration` | `spiceLsp.serverPath`, `spiceLsp.trace.server` |
| `contributes.commands` | `spiceLsp.restartServer` (recommended) |

Example language contribution:

```json
{
  "languages": [{
    "id": "spice",
    "aliases": ["SPICE", "spice"],
    "extensions": [".cir", ".sp", ".net", ".ckt"],
    "configuration": "./language-configuration.json"
  }]
}
```

## language-configuration.json

Teach VS Code comment syntax and line continuation behavior:

```json
{
  "comments": {
    "lineComment": "*"
  },
  "brackets": [["(", ")"]],
  "autoClosingPairs": [
    { "open": "(", "close": ")" }
  ]
}
```

Note: SPICE also uses `;` and `$` comments — full support may require a TextMate grammar (`syntaxes/spice.tmLanguage.json`) for highlighting. MVP can ship with basic `*` comments; Tree-sitter `highlights.scm` can back semantic tokens later.

## extension.ts

Minimal Language Client setup:

```typescript
import * as path from "path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext) {
  const config = vscode.workspace.getConfiguration("spiceLsp");
  const serverPath = config.get<string>("serverPath") || "spice-lsp";

  const serverOptions: ServerOptions = {
    command: serverPath,
    args: [],
    transport: TransportKind.stdio,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "spice" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.{cir,sp,net,ckt}"),
    },
  };

  client = new LanguageClient("spiceLsp", "SPICE Language Server", serverOptions, clientOptions);
  context.subscriptions.push(client.start());

  context.subscriptions.push(
    vscode.commands.registerCommand("spiceLsp.restartServer", async () => {
      await client?.stop();
      await client?.start();
    })
  );
}

export async function deactivate() {
  await client?.stop();
}
```

## Development workflow

### One-time setup

```bash
pixi add nodejs=22          # if not already in pixi.toml
cd editors/vscode
npm install
```

Add devDependencies in `package.json`:

```json
{
  "devDependencies": {
    "@types/vscode": "^1.90.0",
    "@types/node": "^20.0.0",
    "typescript": "^5.0.0",
    "@vscode/vsce": "^3.0.0"
  },
  "dependencies": {
    "vscode-languageclient": "^9.0.0"
  }
}
```

### Daily loop

```bash
# terminal 1 — Rust server
pixi run build

# terminal 2 — extension
cd editors/vscode
npm run watch   # tsc --watch

# VS Code: F5 to launch Extension Development Host
```

Set absolute path to debug binary in Development Host settings:

```json
{
  "spiceLsp.serverPath": "/path/to/spice-lsp/target/debug/spice-lsp"
}
```

Or use `launch.json` `env` / preLaunchTask to build Rust first.

### Verify integration

Follow [Demo and testing](3_demo-and-test.md) VS Code section.

## Bundling the server binary

For distribution, the extension must ship or download `spice-lsp` per platform.

| Strategy | Pros | Cons |
|----------|------|------|
| **User PATH** | Simplest for MVP dev | Poor UX for end users |
| **Setting `serverPath`** | Flexible | Manual configuration |
| **Bundle in `.vsix`** | Works offline | Large artifact; need per-OS builds |
| **Download from GitHub Releases on activate** | Small VSIX | Requires network on first run |

Recommended path:

1. **MVP:** `serverPath` setting + document in README
2. **v0.2:** Download release asset matching `process.platform` + `process.arch`
3. **v0.3:** Optional bundled binary in marketplace package

Example release asset names:

```
spice-lsp-x86_64-unknown-linux-gnu
spice-lsp-aarch64-apple-darwin
spice-lsp-x86_64-pc-windows-msvc.exe
```

## TextMate grammar (optional for MVP)

Syntax highlighting without semantic tokens:

```
editors/vscode/syntaxes/spice.tmLanguage.json
```

Register in `package.json`:

```json
"grammars": [{
  "language": "spice",
  "scopeName": "source.spice",
  "path": "./syntaxes/spice.tmLanguage.json"
}]
```

Tree-sitter-based highlighting via `nvim-treesitter` is separate; VS Code can adopt semantic tokens when the LSP advertises `semanticTokensProvider` (future).

## Publishing

1. Build release binaries in CI for all target triples
2. `cd editors/vscode && npx vsce package`
3. Publish with `vsce publish` (requires Marketplace publisher token)

Pre-publish checklist:

- [ ] `README.md` with demo GIF or screenshot
- [ ] `LICENSE` aligned with repo
- [ ] Server binary strategy documented for users
- [ ] `engines.vscode` set to tested minimum version

## Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| Server not starting | Binary not on PATH | Set `spiceLsp.serverPath` |
| No diagnostics | Wrong language id | Ensure file extension maps to `spice` |
| Stale diagnostics | Server crash | Check Output → SPICE LSP; restart server |
| Wrong binary arch | Download mismatch | Pick correct release asset |

## Beyond VS Code

The same `spice-lsp` binary enables other editors:

| Editor | Integration |
|--------|-------------|
| Neovim | `vim.lsp.enable` or `lspconfig` custom server |
| Helix | `language-server.spice-lsp` in user config |
| Zed | Community extension calling the binary |

VS Code is the reference client; keep editor-specific code out of Rust.

## Related

- [MVP guide](2_mvp.md) — M6 extension milestone
- [Demo and testing](3_demo-and-test.md)
- [LSP features](../5_lsp-features.md)
- [Architecture](../4_architecture.md)
