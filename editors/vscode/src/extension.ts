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

let client: LanguageClient | undefined;
let extensionPath = "";
let output: vscode.OutputChannel | undefined;

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

function createClient(): LanguageClient {
  const config = vscode.workspace.getConfiguration("spiceLsp");
  const serverPath = resolveServerPath(config);
  const hint = missingBinaryHint(serverPath);
  if (hint) {
    throw new Error(hint);
  }

  log(`Starting language server: ${serverPath}`);

  const serverOptions: ServerOptions = {
    command: serverPath,
    args: [],
    transport: TransportKind.stdio,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "spice" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher(
        "**/*.{cir,sp,spf,net,ckt}",
      ),
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
  return error instanceof Error ? error.message : String(error);
}

async function startClient(): Promise<void> {
  client = createClient();
  await client.start();
  log("Language server started.");
}

async function stopClient(): Promise<void> {
  if (!client) {
    return;
  }

  const current = client;
  client = undefined;
  await current.stop();
  log("Language server stopped.");
}

async function restartClient(): Promise<void> {
  await stopClient();
  await startClient();
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  extensionPath = context.extensionPath;
  output = vscode.window.createOutputChannel("SPICE Language Server");
  context.subscriptions.push(output);
  log(`Activating extension from ${extensionPath} (${getPlatformId()})`);

  // Register commands before starting the server so a failed start cannot leave
  // contributed commands unregistered ("not found").
  context.subscriptions.push({
    dispose: () => {
      void stopClient();
    },
  });

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
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (
        event.affectsConfiguration("spiceLsp.serverPath") ||
        event.affectsConfiguration("spiceLsp.trace.server")
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

  try {
    await startClient();
  } catch (error) {
    const message = errorMessage(error);
    log(`Start failed: ${message}`);
    void vscode.window.showErrorMessage(
      `Failed to start SPICE LSP: ${message}. Check Output → SPICE Language Server, then use "SPICE LSP: Restart Server".`,
    );
  }
}

export async function deactivate(): Promise<void> {
  await stopClient();
}
