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
