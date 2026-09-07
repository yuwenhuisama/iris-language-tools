import { dirname } from "node:path";
import * as vscode from "vscode";

export async function runFile(): Promise<void> {
  if (!vscode.workspace.isTrusted) return;
  const document = vscode.window.activeTextEditor?.document;
  if (!document || document.languageId !== "iris" || document.uri.scheme !== "file") {
    await vscode.window.showWarningMessage("Save an Iris file locally before running it.");
    return;
  }
  if (!vscode.workspace.workspaceFolders?.length) {
    await vscode.window.showWarningMessage(`Open the script folder (${dirname(document.uri.fsPath)}) using File > Open Folder, then run the Iris file again.`);
    return;
  }
  if (document.isDirty && !(await document.save())) return;
  const executable = vscode.workspace.getConfiguration("iris").get<string>("executablePath", "iris");
  const task = new vscode.Task(
    { type: "iris", file: document.uri.fsPath },
    vscode.TaskScope.Workspace,
    "Run File on VM",
    "Iris",
    new vscode.ProcessExecution(executable, ["--vm", document.uri.fsPath], { cwd: dirname(document.uri.fsPath) }),
  );
  await vscode.tasks.executeTask(task);
}
