import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Trace,
  TransportKind,
} from "vscode-languageclient/node";
import { DEMO_FILES, DEMO_FOLDER_NAME } from "./demoContent";

let client: LanguageClient | undefined;
let extensionPath = "";
let output: vscode.OutputChannel | undefined;
let dialectStatus: vscode.StatusBarItem | undefined;
/** Serializes start/stop/restart so background activate cannot race Set Dialect. */
let clientLifecycle: Promise<void> = Promise.resolve();

function enqueueClientLifecycle<T>(op: () => Promise<T>): Promise<T> {
  const run = clientLifecycle.then(op, op);
  clientLifecycle = run.then(
    () => undefined,
    () => undefined,
  );
  return run;
}

const DIALECTS = [
  { id: "hspice", label: "HSPICE" },
  { id: "ngspice", label: "Ngspice" },
  { id: "ltspice", label: "LTspice" },
] as const;

type DialectId = (typeof DIALECTS)[number]["id"];

/** Platforms that ship a bundled binary in the Marketplace VSIX. */
const BUNDLED_PLATFORMS = new Set([
  "linux-x64",
  "linux-arm64",
  "darwin-x64",
  "darwin-arm64",
  "win32-x64",
]);

export function getPlatformId(): string {
  return `${process.platform}-${process.arch}`;
}

function log(message: string): void {
  const line = `[spice-lsp] ${message}`;
  output?.appendLine(line);
  console.log(line);
}

function bundledBinaryPath(root: string): string {
  const exe = process.platform === "win32" ? "spice-lsp.exe" : "spice-lsp";
  return path.join(root, "bin", getPlatformId(), exe);
}

function resolveServerPath(config: vscode.WorkspaceConfiguration): string {
  const configured = config.get<string>("serverPath")?.trim();
  if (configured) {
    return configured;
  }

  if (extensionPath) {
    const bundled = bundledBinaryPath(extensionPath);
    if (fs.existsSync(bundled)) {
      return bundled;
    }
  }

  const devBinary = path.resolve(
    __dirname,
    "..",
    "..",
    "..",
    "target",
    "debug",
    process.platform === "win32" ? "spice-lsp.exe" : "spice-lsp",
  );
  if (fs.existsSync(devBinary)) {
    return devBinary;
  }

  const releaseBinary = path.resolve(
    __dirname,
    "..",
    "..",
    "..",
    "target",
    "release",
    process.platform === "win32" ? "spice-lsp.exe" : "spice-lsp",
  );
  if (fs.existsSync(releaseBinary)) {
    return releaseBinary;
  }

  return "spice-lsp";
}

function missingBinaryHint(serverPath: string): string | undefined {
  const configured = vscode.workspace
    .getConfiguration("spiceLsp")
    .get<string>("serverPath")
    ?.trim();
  if (configured) {
    if (!fs.existsSync(configured)) {
      return `Configured spiceLsp.serverPath does not exist: ${configured}`;
    }
    return undefined;
  }

  if (serverPath !== "spice-lsp") {
    return undefined;
  }

  const platformId = getPlatformId();
  if (!BUNDLED_PLATFORMS.has(platformId)) {
    return (
      `No bundled spice-lsp binary for platform ${platformId}. ` +
      `Supported: ${[...BUNDLED_PLATFORMS].join(", ")}. ` +
      `Install spice-lsp on PATH or set spiceLsp.serverPath.`
    );
  }

  return (
    `Bundled spice-lsp binary was not found for ${platformId}. ` +
    `Reinstall the extension, or set spiceLsp.serverPath to a local binary.`
  );
}

function currentDialectId(): DialectId {
  const raw = vscode.workspace
    .getConfiguration("spiceLsp")
    .get<string>("dialect")
    ?.trim()
    .toLowerCase();
  if (raw === "ngspice" || raw === "ltspice" || raw === "hspice") {
    return raw;
  }
  return "hspice";
}

function dialectLabel(id: DialectId): string {
  return DIALECTS.find((d) => d.id === id)?.label ?? id;
}

function updateDialectStatus(): void {
  if (!dialectStatus) {
    return;
  }
  const id = currentDialectId();
  dialectStatus.text = `$(chip) ${dialectLabel(id)}`;
  dialectStatus.tooltip = `SPICE dialect: ${dialectLabel(id)} (click to change)`;
}

function createClient(): LanguageClient {
  const config = vscode.workspace.getConfiguration("spiceLsp");
  const serverPath = resolveServerPath(config);
  const hint = missingBinaryHint(serverPath);
  if (hint) {
    throw new Error(hint);
  }

  const dialect = currentDialectId();
  const libraryPaths = config.get<string[]>("libraryPaths") ?? [];
  const maxDepth = config.get<number>("include.maxDepth") ?? 16;
  log(`Starting language server: ${serverPath} (dialect=${dialect})`);

  const serverOptions: ServerOptions = {
    command: serverPath,
    args: [],
    transport: TransportKind.stdio,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "spice" }],
    synchronize: {
      configurationSection: "spiceLsp",
      fileEvents: vscode.workspace.createFileSystemWatcher(
        "**/*.{cir,sp,spf,net,ckt,inc,lib}",
      ),
    },
    initializationOptions: {
      dialect,
      libraryPaths,
      include: { maxDepth },
    },
    outputChannel: output,
  };

  const languageClient = new LanguageClient(
    "spiceLsp",
    "SPICE Language Server",
    serverOptions,
    clientOptions,
  );

  languageClient.setTrace(parseTraceLevel(config.get<string>("trace.server")));
  return languageClient;
}

function parseTraceLevel(value: string | undefined): Trace {
  switch (value) {
    case "messages":
      return Trace.Messages;
    case "verbose":
      return Trace.Verbose;
    default:
      return Trace.Off;
  }
}

function errorMessage(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  if (/GLIBC_|version `GLIBC/i.test(message)) {
    return (
      `${message}. The bundled Linux spice-lsp binary needs glibc 2.31+ ` +
      `(Ubuntu 20.04 / Debian 11 or newer). Update your OS libraries, or build ` +
      `spice-lsp locally and set spiceLsp.serverPath.`
    );
  }
  return message;
}

async function startClientUnlocked(): Promise<void> {
  if (client) {
    return;
  }
  client = createClient();
  await client.start();
  log("Language server started.");
}

async function stopClientUnlocked(): Promise<void> {
  if (!client) {
    return;
  }

  const current = client;
  client = undefined;
  await current.stop();
  log("Language server stopped.");
}

async function startClient(): Promise<void> {
  await enqueueClientLifecycle(() => startClientUnlocked());
}

async function stopClient(): Promise<void> {
  await enqueueClientLifecycle(() => stopClientUnlocked());
}

async function restartClient(): Promise<void> {
  await enqueueClientLifecycle(async () => {
    await stopClientUnlocked();
    await startClientUnlocked();
  });
}

async function setDialect(): Promise<void> {
  const current = currentDialectId();
  const picked = await vscode.window.showQuickPick(
    DIALECTS.map((d) => ({
      label: d.label,
      description: d.id === current ? `${d.id} (current)` : d.id,
      id: d.id,
    })),
    {
      title: "SPICE LSP: Set Dialect",
      placeHolder: "Select the active SPICE dialect",
    },
  );
  if (!picked) {
    return;
  }

  if (picked.id === current) {
    void vscode.window.showInformationMessage(
      `SPICE dialect already ${picked.label}`,
    );
    return;
  }

  const target = vscode.workspace.workspaceFolders?.length
    ? vscode.ConfigurationTarget.Workspace
    : vscode.ConfigurationTarget.Global;
  // onDidChangeConfiguration restarts the client; avoid a second restart here.
  await vscode.workspace
    .getConfiguration("spiceLsp")
    .update("dialect", picked.id, target);
  updateDialectStatus();
  log(`Dialect set to ${picked.id}`);
  void vscode.window.showInformationMessage(`SPICE dialect: ${picked.label}`);
}

async function resolveDemoParentDir(): Promise<vscode.Uri | undefined> {
  const folders = vscode.workspace.workspaceFolders;
  if (folders && folders.length === 1) {
    return folders[0].uri;
  }
  if (folders && folders.length > 1) {
    const picked = await vscode.window.showWorkspaceFolderPick({
      placeHolder: "Select the folder where spice-lsp-demo will be created",
    });
    return picked?.uri;
  }

  const picked = await vscode.window.showOpenDialog({
    canSelectFiles: false,
    canSelectFolders: true,
    canSelectMany: false,
    openLabel: "Create demo here",
    title: "Select a folder for spice-lsp-demo",
  });
  return picked?.[0];
}

async function createDemoFolder(): Promise<void> {
  const parent = await resolveDemoParentDir();
  if (!parent) {
    return;
  }

  const demoUri = vscode.Uri.joinPath(parent, DEMO_FOLDER_NAME);
  let demoExists = false;
  try {
    await vscode.workspace.fs.stat(demoUri);
    demoExists = true;
  } catch {
    demoExists = false;
  }

  if (demoExists) {
    const choice = await vscode.window.showWarningMessage(
      `"${DEMO_FOLDER_NAME}" already exists in ${parent.fsPath}. Overwrite its demo files?`,
      { modal: true },
      "Overwrite",
      "Open Existing",
    );
    if (!choice) {
      return;
    }
    if (choice === "Open Existing") {
      await openDemoEntry(demoUri);
      return;
    }
  } else {
    await vscode.workspace.fs.createDirectory(demoUri);
  }

  for (const file of DEMO_FILES) {
    const fileUri = vscode.Uri.joinPath(demoUri, file.relativePath);
    await vscode.workspace.fs.writeFile(fileUri, Buffer.from(file.contents, "utf8"));
  }

  log(`Wrote demo folder at ${demoUri.fsPath}`);
  await openDemoEntry(demoUri);
  void vscode.window.showInformationMessage(
    `Created ${DEMO_FOLDER_NAME}. Open same-file.sp or top.sp and press F12 on model/subckt names.`,
  );
}

async function openDemoEntry(demoUri: vscode.Uri): Promise<void> {
  const readme = vscode.Uri.joinPath(demoUri, "README.md");
  const top = vscode.Uri.joinPath(demoUri, "top.sp");
  try {
    await vscode.window.showTextDocument(top, { preview: false });
  } catch (error) {
    log(`Could not open top.sp: ${errorMessage(error)}`);
  }
  try {
    await vscode.window.showTextDocument(readme, {
      preview: false,
      viewColumn: vscode.ViewColumn.Beside,
    });
  } catch (error) {
    log(`Could not open demo README: ${errorMessage(error)}`);
  }
}

async function startClientInBackground(): Promise<void> {
  try {
    await startClient();
  } catch (error) {
    const message = errorMessage(error);
    log(`Start failed: ${message}`);
    void vscode.window.showErrorMessage(
      `Failed to start SPICE LSP: ${message}. Check Output → SPICE Language Server, then use "SPICE LSP: Restart Server" or "SPICE LSP: Set Dialect…".`,
    );
  }
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  extensionPath = context.extensionPath;
  output = vscode.window.createOutputChannel("SPICE Language Server");
  context.subscriptions.push(output);
  log(`Activating extension from ${extensionPath} (${getPlatformId()})`);

  // Register commands before any await so onCommand activation (Set Dialect /
  // Restart Server / Create Demo Folder) completes promptly. Awaiting client.start()
  // here made VS Code report "command not found" when the language server was slow or hung.
  context.subscriptions.push(
    vscode.commands.registerCommand("spiceLsp.restartServer", async () => {
      log("Restart Server command invoked.");
      try {
        await restartClient();
        void vscode.window.showInformationMessage("SPICE LSP restarted.");
      } catch (error) {
        const message = errorMessage(error);
        log(`Restart failed: ${message}`);
        void vscode.window.showErrorMessage(
          `Failed to restart SPICE LSP: ${message}`,
        );
      }
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("spiceLsp.setDialect", async () => {
      await setDialect();
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("spiceLsp.createDemoFolder", async () => {
      log("Create Demo Folder command invoked.");
      try {
        await createDemoFolder();
      } catch (error) {
        const message = errorMessage(error);
        log(`Create demo folder failed: ${message}`);
        void vscode.window.showErrorMessage(
          `Failed to create spice-lsp-demo: ${message}`,
        );
      }
    }),
  );

  dialectStatus = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100,
  );
  dialectStatus.command = "spiceLsp.setDialect";
  updateDialectStatus();
  dialectStatus.show();
  context.subscriptions.push(dialectStatus);

  context.subscriptions.push({
    dispose: () => {
      void stopClient();
    },
  });

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (event.affectsConfiguration("spiceLsp.dialect")) {
        updateDialectStatus();
      }
      if (
        event.affectsConfiguration("spiceLsp.serverPath") ||
        event.affectsConfiguration("spiceLsp.trace.server") ||
        event.affectsConfiguration("spiceLsp.dialect") ||
        event.affectsConfiguration("spiceLsp.libraryPaths") ||
        event.affectsConfiguration("spiceLsp.include.maxDepth")
      ) {
        try {
          await restartClient();
        } catch (error) {
          const message = errorMessage(error);
          log(`Config restart failed: ${message}`);
          void vscode.window.showErrorMessage(
            `Failed to restart SPICE LSP: ${message}`,
          );
        }
      }
    }),
  );

  void startClientInBackground();
}

export async function deactivate(): Promise<void> {
  await stopClient();
}
