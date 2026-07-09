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

The Marketplace extension ships a **platform-specific binary** inside the `.vsix` under `bin/<platform>-<arch>/`:

| Platform id | OS / arch |
|-------------|-----------|
| `linux-x64` | Linux x86_64 |
| `linux-arm64` | Linux ARM64 |
| `darwin-x64` | macOS Intel |
| `darwin-arm64` | macOS Apple Silicon |
| `win32-x64` | Windows x64 |

At activation, the extension resolves the binary in this order:

1. `spiceLsp.serverPath` setting (if set)
2. Bundled binary at `bin/<platform>-<arch>/spice-lsp` inside the extension
3. Local dev paths under `target/debug` or `target/release` (F5 from this repo)
4. `spice-lsp` on `PATH`

### Package locally

Build a release binary for the current platform and create a `.vsix`:

```bash
pixi run build
pixi run ext-package
# output: editors/vscode/spice-lsp-0.2.0.vsix
```

Install side-loaded:

```bash
code --install-extension editors/vscode/spice-lsp-0.2.0.vsix
```

### CI release workflow

The [Release VS Code extension](../../.github/workflows/release-vscode.yml) workflow runs on every push to `main` (and on manual `workflow_dispatch` / `vscode-v*` tags):

1. Bumps the patch version in `editors/vscode/package.json` and commits it to `main`
2. Cross-compiles `spice-lsp` for all supported platform ids
3. Assembles a single `.vsix` containing every platform binary
4. Uploads the VSIX as a GitHub Actions artifact
5. Creates a GitHub Release tagged `vscode-v<version>`
6. Publishes to the VS Code Marketplace from the same package job (`VSCE_PAT` required)

| Strategy | Pros | Cons |
|----------|------|------|
| **User PATH** | Simplest for local dev | Poor UX for end users |
| **Setting `serverPath`** | Flexible | Manual configuration |
| **Bundle in `.vsix`** | Works offline; Marketplace default | Larger artifact; CI builds all platforms |
| **Download from GitHub Releases on activate** | Small VSIX | Requires network on first run |

The Marketplace release uses **bundle in `.vsix`**.

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

### One-time Marketplace setup

Do this once before the first CI publish succeeds:

1. Sign in to the [Visual Studio Marketplace publisher management](https://marketplace.visualstudio.com/manage) page with a Microsoft account.
2. Create a publisher whose **Publisher ID** matches `editors/vscode/package.json` (`AmirhosseinDavoody` in this repo). The ID is permanent and must match exactly.
3. Create a Personal Access Token in **Azure DevOps** (not [portal.azure.com](https://portal.azure.com)):
   1. Open [https://dev.azure.com](https://dev.azure.com) and sign in with the **same Microsoft account** used for the Marketplace publisher.
   2. If prompted, create a free Azure DevOps organization (any name is fine; it is only a container for the PAT).
   3. Click your profile avatar (top right) → **Personal access tokens**  
      Direct link: [https://dev.azure.com/_usersSettings/tokens](https://dev.azure.com/_usersSettings/tokens)
   4. **+ New Token**:
      - Name: e.g. `vscode-marketplace`
      - Organization: **All accessible organizations**
      - Expiration: choose a duration you are willing to rotate
      - Scopes: **Custom defined** → enable **Marketplace → Manage**
   5. Create and **copy the token immediately** (it is shown once)
4. In the GitHub repo: **Settings → Secrets and variables → Actions → New repository secret**
   - Name: `VSCE_PAT`
   - Value: the Azure DevOps PAT from step 3
5. Confirm Marketplace listing metadata is ready in `editors/vscode/`:
   - `README.md` (Marketplace landing page)
   - `LICENSE` (`MIT` matches `package.json`)
   - `publisher`, `displayName`, `description`, `engines.vscode`

Optional local dry-run before relying on CI:

```bash
pixi run build
pixi run ext-package
cd editors/vscode
npx vsce package --no-dependencies   # validates packaging without publishing
```

### Release from CI (automatic)

Every push to `main` runs **Release VS Code extension**:

1. Patch-bumps `editors/vscode/package.json` (e.g. `0.2.0` → `0.2.1`)
2. Commits `chore(vscode): bump extension to <version>` (via `GITHUB_TOKEN`, which does not re-trigger the workflow)
3. Builds platform binaries, packages the VSIX, creates GitHub Release `vscode-v<version>`, and runs `vsce publish`

Manual options:

- **Actions tab → Release VS Code extension → Run workflow** — optional bump + publish flags
- Push a tag `vscode-v*` from your machine to publish the version already in `package.json` (no auto-bump)

The workflow always uploads the `.vsix` as an Actions artifact.

### Release manually

```bash
pixi run build
pixi run ext-package
cd editors/vscode
# bump version first if this version was already published
npm version patch --no-git-tag-version
npx vsce publish --no-dependencies   # requires VSCE_PAT in the environment
```

Pre-publish checklist:

- [ ] Publisher ID matches `package.json` `publisher` field
- [ ] `VSCE_PAT` repository secret configured
- [ ] `README.md` describes bundled-binary behavior for end users
- [ ] `LICENSE` aligned with repo (`MIT` in `package.json`)
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
