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

export function getPlatformId(): string {
  return `${process.platform}-${process.arch}`;
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

function createClient(): LanguageClient {
  const config = vscode.workspace.getConfiguration("spiceLsp");
  const serverPath = resolveServerPath(config);

  const serverOptions: ServerOptions = {
    command: serverPath,
    args: [],
    transport: TransportKind.stdio,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "spice" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher(
        "**/*.{cir,sp,net,ckt}",
      ),
    },
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
}

async function stopClient(): Promise<void> {
  if (!client) {
    return;
  }

  const current = client;
  client = undefined;
  await current.stop();
}

async function restartClient(): Promise<void> {
  await stopClient();
  await startClient();
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  extensionPath = context.extensionPath;

  // Register commands and subscriptions before starting the server so a failed
  // start cannot leave contributed commands unregistered ("not found").
  context.subscriptions.push({
    dispose: () => {
      void stopClient();
    },
  });

  context.subscriptions.push(
    vscode.commands.registerCommand("spiceLsp.restartServer", async () => {
      try {
        await restartClient();
      } catch (error) {
        void vscode.window.showErrorMessage(
          `Failed to restart SPICE LSP: ${errorMessage(error)}`,
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
          void vscode.window.showErrorMessage(
            `Failed to restart SPICE LSP: ${errorMessage(error)}`,
          );
        }
      }
    }),
  );

  try {
    await startClient();
  } catch (error) {
    void vscode.window.showErrorMessage(
      `Failed to start SPICE LSP: ${errorMessage(error)}. Use "SPICE LSP: Restart Server" after fixing the server path.`,
    );
  }
}

export async function deactivate(): Promise<void> {
  await stopClient();
}
