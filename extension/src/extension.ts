import * as vscode from "vscode";
import { CloseAction, ErrorAction } from "vscode-languageclient/node";
import { runFile } from "./run-file";
import { ServerClient, StartupInterrupted } from "./server-client";

let client: ServerClient | undefined;
let output: vscode.OutputChannel;
let fileEvents: vscode.FileSystemWatcher[] = [];
let pending: Promise<void> | undefined;
let deactivating = false;
let restarting: Promise<void> | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  if (!vscode.workspace.isTrusted) return;
  output = vscode.window.createOutputChannel("Iris Language Server");
  context.subscriptions.push(
    output,
    vscode.commands.registerCommand("iris.runFile", runFile),
    vscode.commands.registerCommand("iris.restartLanguageServer", restart),
  );
  pending = start().finally(() => { pending = undefined; });
  await pending;
}

function restart(): Promise<void> {
  if (deactivating || !vscode.workspace.isTrusted) return Promise.resolve();
  if (restarting) return restarting;
  if (pending) {
    client?.cancelStartup();
    restarting = pending.then(() => {
      pending = start().finally(() => { pending = undefined; });
      return pending;
    }).finally(() => { restarting = undefined; });
    return restarting;
  }
  pending = start().finally(() => { pending = undefined; });
  restarting = pending.finally(() => { restarting = undefined; });
  return restarting;
}

async function disposeClient(): Promise<void> {
  try {
    await client?.dispose();
  } finally {
    client = undefined;
    for (const watcher of fileEvents) watcher.dispose();
    fileEvents = [];
  }
}

async function start(): Promise<void> {
  let command: string | undefined;
  try {
    await disposeClient();
    if (deactivating || !vscode.workspace.isTrusted) return;
    command = vscode.workspace.getConfiguration("iris").get<string>("serverPath", "iris-lsp");
    output.appendLine(`Starting language server: ${JSON.stringify({ command })}`);
    for (const pattern of ["**/*.ir", "**/*.iris", "**/iris.toml"]) {
      fileEvents.push(vscode.workspace.createFileSystemWatcher(pattern));
    }
    client = new ServerClient(command, {
      documentSelector: [{ scheme: "file", language: "iris" }, { scheme: "untitled", language: "iris" }],
      outputChannel: output,
      synchronize: { fileEvents },
      initializationFailedHandler: () => false,
      errorHandler: {
        error: () => ({ action: ErrorAction.Shutdown }),
        closed: () => {
          if (!deactivating) {
            output.appendLine("Language server connection closed. Use Iris: Restart Language Server to retry.");
          }
          return { action: CloseAction.DoNotRestart };
        },
      },
      middleware: {
        provideDocumentFormattingEdits: async (document, options, token, next) => {
          const edits = await next(document, options, token);
          if (!edits?.length) return edits;
          if (edits.some(edit => edit.newText.includes("\r"))) {
            output.appendLine("Formatting skipped: this editor cannot normalize exterior line endings without changing protected content.");
            return [];
          }
          if (document.eol === vscode.EndOfLine.LF) return edits;
          return [...edits, vscode.TextEdit.setEndOfLine(vscode.EndOfLine.LF)];
        },
      },
    });
    await client.start();
    if (deactivating) return;
    const result = client.initializeResult;
    const capabilities = {
      formatting: Boolean(result?.capabilities.documentFormattingProvider),
      definition: Boolean(result?.capabilities.definitionProvider),
      references: Boolean(result?.capabilities.referencesProvider),
      completion: Boolean(result?.capabilities.completionProvider),
      inlayHints: Boolean(result?.capabilities.inlayHintProvider),
      hover: Boolean(result?.capabilities.hoverProvider),
    };
    output.appendLine(`Language server initialized: ${JSON.stringify({ serverInfo: result?.serverInfo ?? null, capabilities })}`);
    const missing = Object.entries(capabilities).filter(([, available]) => !available).map(([name]) => name);
    if (missing.length) {
      const warning = `Iris server is missing capabilities: ${missing.join(", ")}. It may be a stale or wrong server. Check iris.serverPath, rebuild iris-lsp, then restart. Available features remain enabled.`;
      output.appendLine(warning);
      void vscode.window.showWarningMessage(warning);
    }
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    const interrupted = error instanceof StartupInterrupted && !error.timedOut;
    if (!interrupted) output.appendLine(`Language server startup failed: ${JSON.stringify({ command, error: message })}`);
    try {
      await disposeClient();
    } catch (cleanupError: unknown) {
      output.appendLine(`Language server cleanup failed: ${cleanupError instanceof Error ? cleanupError.message : String(cleanupError)}`);
    }
    if (!deactivating && !interrupted) void offerRecovery();
  }
}

async function offerRecovery(): Promise<void> {
  const action = await vscode.window.showErrorMessage(
    "Iris language server could not start. Check Output, build iris-lsp and set iris.serverPath to your executable, then retry.",
    "Show Output", "Open Settings", "Retry",
  );
  if (deactivating) return;
  switch (action) {
    case "Show Output": output.show(); break;
    case "Open Settings": await vscode.commands.executeCommand("workbench.action.openSettings", "iris.serverPath"); break;
    case "Retry":
      await pending;
      await restart();
      break;
    case undefined: break;
  }
}

export async function deactivate(): Promise<void> {
  deactivating = true;
  client?.cancelStartup();
  await pending;
  await disposeClient();
}
