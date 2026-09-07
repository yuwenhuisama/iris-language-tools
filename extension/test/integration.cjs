const assert = require('node:assert/strict');
const vscode = require('vscode');

function diagnosticsWhen(uri, predicate) {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => { subscription.dispose(); reject(new Error('Timed out waiting for Iris diagnostics')); }, 60000);
    const subscription = vscode.languages.onDidChangeDiagnostics(event => {
      if (!event.uris.some(value => value.toString() === uri.toString())) return;
      const diagnostics = vscode.languages.getDiagnostics(uri);
      if (predicate(diagnostics)) {
        clearTimeout(timeout);
        subscription.dispose();
        resolve(diagnostics);
      }
    });
  });
}

exports.run = async function () {
  const extension = vscode.extensions.getExtension('iris-local.iris-language-tools');
  assert.ok(extension);
  await extension.activate();
  await require('./semantic-integration.cjs').verifySemantics();
  await require('./editing.test.cjs').run();
  await require('./formatting-integration.cjs').verifyFormatting();
  await require('./runtime-formatting.cjs').verifyRuntimeFormatting();
  if (process.env.IRIS_TEST_EMPTY === '1') {
    await verifyEmptyWorkspace();
    return;
  }
  const uri = vscode.Uri.parse('untitled:integration.iris');
  const errors = diagnosticsWhen(uri, values => values.length === 1);
  const document = await vscode.workspace.openTextDocument(uri);
  await vscode.languages.setTextDocumentLanguage(document, 'iris');
  const editor = await vscode.window.showTextDocument(document);
  await editor.edit(edit => edit.insert(new vscode.Position(0, 0), '// heading\r\nlet value = "😀"; \\'));
  const diagnostics = await errors;
  assert.equal(diagnostics[0].code, 'LEX_BAD_CONTINUATION');
  assert.equal(diagnostics[0].range.start.line, 1);
  assert.equal(diagnostics[0].range.start.character, 18);
  const cleared = diagnosticsWhen(uri, values => values.length === 0);
  await editor.edit(edit => edit.replace(new vscode.Range(document.positionAt(0), document.positionAt(document.getText().length)), 'let clean = 1'));
  await cleared;
  assert.equal(document.isDirty, true);
  const completions = await vscode.commands.executeCommand('vscode.executeCompletionItemProvider', uri, new vscode.Position(0, 0));
  assert.ok(completions.items.some(item => item.label === 'yield'));
  assert.ok(completions.items.some(item => item.label === 'typeof'));
  console.log('PASS: real extension host activation, Unicode/CRLF diagnostics, unsaved clearing and keyword completion');
};

async function verifyEmptyWorkspace() {
  const fs = require('node:fs/promises');
  const os = require('node:os');
  const path = require('node:path');
  assert.equal(vscode.workspace.workspaceFolders?.length ?? 0, 0);
  const folder = await fs.mkdtemp(path.join(os.tmpdir(), 'iris empty workspace '));
  const warning = vscode.window.showWarningMessage;
  const messages = [];
  const tasks = [];
  const subscription = vscode.tasks.onDidStartTask(event => tasks.push(event));
  try {
    const file = path.join(folder, 'test.iris');
    await fs.writeFile(file, 'print("saved")');
    const document = await vscode.workspace.openTextDocument(file);
    const editor = await vscode.window.showTextDocument(document);
    await editor.edit(edit => edit.insert(new vscode.Position(0, 0), '// unsaved\n'));
    vscode.window.showWarningMessage = async message => { messages.push(message); };
    await vscode.commands.executeCommand('iris.runFile');
    assert.equal(messages.length, 1);
    assert.equal(tasks.length, 0);
    assert.equal(document.isDirty, true);
    assert.equal(await fs.readFile(file, 'utf8'), 'print("saved")');
    await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
    console.log('PASS: real empty workspace refuses unsupported run before saving or launching a task');
  } finally {
    vscode.window.showWarningMessage = warning;
    subscription.dispose();
    await fs.rm(folder, { recursive: true, force: true });
  }
}
