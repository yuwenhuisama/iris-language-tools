import * as vscode from "vscode";
import { LanguageClient } from "vscode-languageclient/node";
import { runFile } from "./run-file";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  if (!vscode.workspace.isTrusted) return;
  context.subscriptions.push(vscode.commands.registerCommand("iris.runFile", runFile));
  const command = vscode.workspace.getConfiguration("iris").get<string>("serverPath", "iris-lsp");
  const output = vscode.window.createOutputChannel("Iris Language Server");
  const fileEvents = [
    vscode.workspace.createFileSystemWatcher("**/*.iris"),
    vscode.workspace.createFileSystemWatcher("**/iris.toml"),
  ];
  context.subscriptions.push(output, ...fileEvents);
  client = new LanguageClient("iris", "Iris Language Server", { command }, {
    documentSelector: [{ scheme: "file", language: "iris" }, { scheme: "untitled", language: "iris" }],
    outputChannel: output,
    synchronize: { fileEvents },
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
  try {
    await client.start();
  } catch (error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    output.appendLine(`Could not start ${command}: ${message}`);
    await vscode.window.showErrorMessage("Iris language server failed to start. Build iris-lsp, set iris.serverPath, then Reload Window. See Iris Language Server output.");
  }
}

export async function deactivate(): Promise<void> {
  await client?.dispose();
  client = undefined;
}
