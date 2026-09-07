const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const vscode = require('vscode');

exports.verifyFormatting = async function () {
  const source = 'module Demo {\r\npublic module fun hello() {\r\nprint("😀")\r\n}\r\n}';
  const expected = 'module Demo {\r\n  public module fun hello() {\r\n    print("😀")\r\n  }\r\n}';
  const document = await vscode.workspace.openTextDocument({ language: 'iris', content: source });
  await vscode.window.showTextDocument(document);
  const edits = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', document.uri, { tabSize: 2, insertSpaces: true });
  assert.ok(edits?.length > 0, 'Iris must provide document formatting');
  const edit = new vscode.WorkspaceEdit();
  edit.set(document.uri, edits);
  assert.equal(await vscode.workspace.applyEdit(edit), true);
  assert.equal(document.getText(), expected);
  const repeated = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', document.uri, { tabSize: 2, insertSpaces: true });
  assert.equal(repeated?.length ?? 0, 0, 'Formatting must be idempotent');
  await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');

  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'iris format save '));
  try {
    const file = path.join(root, 'format.iris');
    await fs.writeFile(file, '');
    const saved = await vscode.workspace.openTextDocument(file);
    const editor = await vscode.window.showTextDocument(saved);
    const config = vscode.workspace.getConfiguration('editor', { uri: saved.uri, languageId: 'iris' });
    await config.update('defaultFormatter', 'iris-local.iris-language-tools', vscode.ConfigurationTarget.Global, true);
    await config.update('formatOnSave', true, vscode.ConfigurationTarget.Global, true);
    await config.update('detectIndentation', false, vscode.ConfigurationTarget.Global, true);
    await config.update('tabSize', 2, vscode.ConfigurationTarget.Global, true);
    await config.update('insertSpaces', true, vscode.ConfigurationTarget.Global, true);
    editor.options = { tabSize: 2, insertSpaces: true };
    await editor.edit(builder => builder.insert(new vscode.Position(0, 0), 'class C {\nlet x = 1\n}'));
    assert.equal(await saved.save(), true);
    assert.equal(await fs.readFile(file, 'utf8'), 'class C {\n  let x = 1\n}');
    assert.equal(saved.isDirty, false);

    const incomplete = 'class C {\nlet x = 1';
    await editor.edit(builder => builder.replace(new vscode.Range(saved.positionAt(0), saved.positionAt(saved.getText().length)), incomplete));
    assert.equal(await saved.save(), true);
    assert.equal(await fs.readFile(file, 'utf8'), incomplete, 'Unclosed delimiters must not be changed on save');
    await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
    await config.update('formatOnSave', undefined, vscode.ConfigurationTarget.Global, true);
    console.log('PASS: unsaved CRLF/Unicode formatting, idempotence, format-on-save and incomplete-source no-op');
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
};
