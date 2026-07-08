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

function resolveServerPath(config: vscode.WorkspaceConfiguration): string {
  const configured = config.get<string>("serverPath")?.trim();
  if (configured) {
    return configured;
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

async function startClient(): Promise<void> {
  client = createClient();
  await client.start();
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  await startClient();

  context.subscriptions.push({
    dispose: () => {
      void client?.stop();
    },
  });

  context.subscriptions.push(
    vscode.commands.registerCommand("spiceLsp.restartServer", async () => {
      if (client) {
        await client.stop();
      }
      await startClient();
    }),
  );

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (
        event.affectsConfiguration("spiceLsp.serverPath") ||
        event.affectsConfiguration("spiceLsp.trace.server")
      ) {
        if (client) {
          await client.stop();
        }
        await startClient();
      }
    }),
  );
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
  }
}
