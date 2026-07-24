# VS Code Integration

The **VS Code extension** launches the Rust language server and provides a first-class editing experience for SPICE netlists.

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
├── demo/                    # HSPICE samples copied by Create Demo Folder
├── src/
│   ├── extension.ts
│   └── demoContent.ts
├── language-configuration.json
└── README.md                # Marketplace-facing extension readme
```

## package.json

Key fields:

| Field | Purpose |
|-------|---------|
| `engines.vscode` | Minimum VS Code version |
| `activationEvents` | `onLanguage:spice`, `onCommand:spiceLsp.restartServer`, `onCommand:spiceLsp.setDialect`, `onCommand:spiceLsp.createDemoFolder` |
| `main` | `./out/extension.js` (esbuild bundle) |
| `contributes.languages` | Register `spice` language id and file extensions |
| `contributes.configuration` | `spiceLsp.serverPath`, `spiceLsp.trace.server`, `spiceLsp.dialect` |
| `contributes.commands` | `spiceLsp.restartServer`, `spiceLsp.setDialect`, `spiceLsp.createDemoFolder` — register first in `activate`; do not await LSP start before returning |

Example language contribution:

```json
{
  "languages": [{
    "id": "spice",
    "aliases": ["SPICE", "spice"],
    "extensions": [".cir", ".sp", ".spf", ".net", ".ckt"],
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

Comment toggle uses `*` (`language-configuration.json` allows only one `lineComment`). `;` and `$` comments are highlighted by the TextMate grammar (`syntaxes/spice.tmLanguage.json`). Tree-sitter `highlights.scm` can back semantic tokens later.

## extension.ts

Minimal Language Client setup. Register **palette commands** **before** any `await`, then start the client in the background. If `activate` awaits a slow/hung `client.start()`, VS Code times out `onCommand` activation and reports `command 'spiceLsp.setDialect' not found` (same for Restart Server / Create Demo Folder):

```typescript
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

async function startClient(serverPath: string) {
  // TransportKind.stdio makes the client append `--stdio` to the process args.
  // spice-lsp accepts that flag (stdio is the only transport).
  const serverOptions: ServerOptions = {
    command: serverPath,
    args: [],
    transport: TransportKind.stdio,
  };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "spice" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.{cir,sp,spf,net,ckt}"),
    },
  };
  client = new LanguageClient("spiceLsp", "SPICE Language Server", serverOptions, clientOptions);
  await client.start();
}

export async function activate(context: vscode.ExtensionContext) {
  const config = vscode.workspace.getConfiguration("spiceLsp");
  const serverPath = config.get<string>("serverPath") || "spice-lsp";

  context.subscriptions.push(
    vscode.commands.registerCommand("spiceLsp.restartServer", async () => {
      await client?.stop();
      await startClient(serverPath);
    }),
    vscode.commands.registerCommand("spiceLsp.setDialect", async () => {
      /* QuickPick → update spiceLsp.dialect → restart client */
    }),
    vscode.commands.registerCommand("spiceLsp.createDemoFolder", async () => {
      /* Write spice-lsp-demo/ with sample .sp netlists under the workspace folder */
    }),
  );

  void startClient(serverPath).catch((error) => {
    const message = error instanceof Error ? error.message : String(error);
    void vscode.window.showErrorMessage(`Failed to start SPICE LSP: ${message}`);
  });
}

export async function deactivate() {
  await client?.stop();
}
```

### Create Demo Folder

**SPICE LSP: Create Demo Folder** copies the templates from `editors/vscode/demo/` into a `spice-lsp-demo/` directory under the opened workspace folder (or a folder you pick if none is open). It also sets `spiceLsp.dialect` to **hspice**.

| File | Purpose |
|------|---------|
| `same-file.sp` | HSPICE `.param` / `.option` / `.model` / `.subckt` — **F12** on `buffer` / `nch` stays in-file |
| `models.sp` | Shared models and subcircuits |
| `top.sp` | `.include 'models.sp'` — **F12** on `nch` / `inverter` / `buffer` jumps across files |
| `corners.lib` | HSPICE `.lib` / `.endl` corner sections |
| `top-lib.sp` | `.lib 'corners.lib' TT` — **F12** on the path opens `corners.lib`; on `TT` jumps to `.lib TT`; on `nch_tt` / `pch_tt` jumps to the model |
| `README.md` | Short walkthrough |

If the folder already exists, the command offers **Overwrite** or **Open Existing**. Templates live in the extension package under `demo/` so Marketplace installs ship the same samples.
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
npm run watch   # esbuild --watch

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

Follow [Demo and testing](2_demo-and-test.md) VS Code section.

## Bundling the server binary

The Marketplace extension ships a **platform-specific binary** inside the `.vsix` under `bin/<platform>-<arch>/`:

| Platform id | OS / arch | Notes |
|-------------|-----------|-------|
| `linux-x64` | Linux x86_64 | Linked for **glibc 2.31+** (Ubuntu 20.04 / Debian 11+) via Zig |
| `linux-arm64` | Linux ARM64 | Same glibc 2.31 floor |
| `darwin-x64` | macOS Intel | |
| `darwin-arm64` | macOS Apple Silicon | |
| `win32-x64` | Windows x64 | |

There is **no** `win32-arm64` bundle today. Unsupported platforms must set `spiceLsp.serverPath` or put `spice-lsp` on `PATH`.

Linux CI builds use `scripts/zig-cc-*.sh` so binaries from `ubuntu-latest` (glibc 2.39) still load on hosts with glibc 2.31. A plain `cargo build` on a newer distro may require a newer glibc — use the Zig wrappers for release artifacts.

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

## TextMate grammar

Syntax highlighting ships as:

```
editors/vscode/syntaxes/spice.tmLanguage.json
```

Registered in `package.json`:

```json
"grammars": [{
  "language": "spice",
  "scopeName": "source.spice",
  "path": "./syntaxes/spice.tmLanguage.json"
}]
```

The grammar colors `*` / `;` / `$` comments, `.` directives, instance lines, and numeric literals. Tree-sitter-based highlighting via `nvim-treesitter` is separate; VS Code can adopt semantic tokens when the LSP advertises `semanticTokensProvider` (future).

Marketplace listing icon: `editors/vscode/media/icon.png` (`package.json` `"icon"` field).

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
   - `README.md` (Marketplace landing page — include a **Quick start** so users know what to do after install)
   - `LICENSE` (`MIT` matches `package.json`)
   - `publisher`, `displayName`, `description`, `engines.vscode`

Optional local dry-run before relying on CI:

```bash
pixi run build
pixi run ext-package
# output: editors/vscode/spice-lsp-<version>.vsix
# packaging fails if the esbuild bundle is missing LanguageClient or terminateProcess.sh
```

The extension is **esbuild-bundled** (`npm run compile` → `out/extension.js` with `vscode-languageclient` inlined). Package and publish with `vsce … --no-dependencies` so the VSIX does not ship `node_modules`.

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
npx vsce publish --no-dependencies --packagePath "$(ls -t *.vsix | head -1)"   # requires VSCE_PAT
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
| Server not starting | Binary not on PATH / wrong `serverPath` / unsupported platform | Set `spiceLsp.serverPath`, then **SPICE LSP: Restart Server**; check Output → SPICE Language Server |
| `unexpected argument '--stdio'` / server exits code 2 | Bundled binary predates the `--stdio` CLI flag (needed by `TransportKind.stdio`) | Update the Marketplace extension, or build from source and set `spiceLsp.serverPath` |
| `version 'GLIBC_2.3x' not found` | Host glibc older than the binary | Update to a Marketplace build linked for glibc 2.31+, or build locally and set `spiceLsp.serverPath` |
| `spiceLsp.restartServer` / `spiceLsp.setDialect` / `spiceLsp.createDemoFolder` not found | Extension never activated, activate hung on LSP start, or Marketplace build predates the command (`setDialect` needs **≥ 0.2.10**; Create Demo Folder is newer) | Update the extension; reload the window; open a `.cir`/`.sp` file or run the command (auto-activates). Prefer builds that register commands before awaiting `client.start()` |
| No **SPICE Language Server** in Output | Extension did not activate | Open a SPICE file or run **SPICE LSP: Restart Server** / **Set Dialect…**; check **Developer: Show Running Extensions** for activation errors |
| Extension activates with module errors | Unbundled VSIX missing `node_modules` | Use an esbuild-bundled release (`vsce package --no-dependencies` after `npm run compile`) |
| No diagnostics | Wrong language id | Ensure file extension maps to `spice` |
| Stale diagnostics | Server crash | Check Output → SPICE Language Server; restart server |
| Wrong binary arch | Download mismatch / unsupported platform | Pick correct release asset or build from source |

## Beyond VS Code

The same `spice-lsp` binary enables other editors:

| Editor | Integration |
|--------|-------------|
| Neovim | `vim.lsp.enable` or `lspconfig` custom server |
| Helix | `language-server.spice-lsp` in user config |
| Zed | Community extension calling the binary |

VS Code is the reference client; keep editor-specific code out of Rust.

## Related

- [Demo and testing](2_demo-and-test.md)
- [LSP features](../5_lsp-features.md)
- [Architecture](../4_architecture.md)
