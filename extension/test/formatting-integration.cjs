const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const vscode = require('vscode');

exports.verifyFormatting = async function () {
  const source = 'module Demo {\npublic module fun hello() {\nprint("😀")\n}\n}';
  const expected = 'module Demo {\n  public module fun hello() {\n    print("😀")\n  }\n}\n';
  const document = await vscode.workspace.openTextDocument({ language: 'iris', content: source });
  await vscode.window.showTextDocument(document);
  const edits = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', document.uri, { tabSize: 8, insertSpaces: false });
  assert.ok(edits?.length > 0, 'Iris must provide document formatting');
  const edit = new vscode.WorkspaceEdit();
  edit.set(document.uri, edits);
  assert.equal(await vscode.workspace.applyEdit(edit), true);
  assert.equal(document.getText(), expected);
  assert.equal(document.eol, vscode.EndOfLine.LF);
  const repeated = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', document.uri, { tabSize: 2, insertSpaces: true });
  assert.equal(repeated?.length ?? 0, 0, 'Formatting must be idempotent');
  await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');

  const protectedSource = 'let text = """\r\n  original\r\n  """\r\n';
  const protectedDocument = await vscode.workspace.openTextDocument({ language: 'iris', content: protectedSource });
  const protectedEdits = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', protectedDocument.uri, { tabSize: 2, insertSpaces: true });
  assert.equal(protectedEdits?.length ?? 0, 0, 'Do not normalize protected literal CRLF through the editor model');
  assert.equal(protectedDocument.getText(), protectedSource);

  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'iris format save '));
  try {
    const file = path.join(root, 'format.iris');
    await fs.writeFile(file, '// heading\r\n');
    const saved = await vscode.workspace.openTextDocument(file);
    const editor = await vscode.window.showTextDocument(saved);
    const config = vscode.workspace.getConfiguration('editor', { uri: saved.uri, languageId: 'iris' });
    await config.update('defaultFormatter', 'iris-local.iris-language-tools', vscode.ConfigurationTarget.Global, true);
    await config.update('formatOnSave', true, vscode.ConfigurationTarget.Global, true);
    await config.update('detectIndentation', false, vscode.ConfigurationTarget.Global, true);
    await config.update('tabSize', 2, vscode.ConfigurationTarget.Global, true);
    await config.update('insertSpaces', true, vscode.ConfigurationTarget.Global, true);
    editor.options = { tabSize: 2, insertSpaces: true };
    assert.equal(saved.eol, vscode.EndOfLine.CRLF);
    await editor.edit(builder => builder.replace(new vscode.Range(saved.positionAt(0), saved.positionAt(saved.getText().length)), 'class C {\nlet x = 1\n}'));
    assert.equal(await saved.save(), true);
    assert.equal(await fs.readFile(file, 'utf8'), 'class C {\n  let x = 1\n}\n');
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
