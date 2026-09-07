const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const vscode = require('vscode');
const { eventually, provider, position, range, locations, location, hints, replace } = require('./semantic-support.cjs');
const { assertHover } = require('./hover-integration.cjs');

exports.run = async function () {
  const extension = vscode.extensions.getExtension('iris-local.iris-language-tools');
  assert.ok(extension);
  assert.equal(extension.isActive, false, 'Fresh host must not pre-activate Iris');
  const source = 'module Main { let value=1; value; fun read(value) { value } }';
  const suffix = process.env.IRIS_STARTUP_SUFFIX;
  assert.ok(['.ir', '.iris'].includes(suffix));
  const file = path.join(process.env.IRIS_STARTUP_WORKSPACE, `automatic${suffix}`);
  await fs.writeFile(file, source);
  const document = await vscode.workspace.openTextDocument(file);
  const editor = await vscode.window.showTextDocument(document);
  assert.equal(document.languageId, 'iris');
  await eventually(`automatic ${suffix} activation`, async () => assert.equal(extension.isActive, true));

  if (process.env.IRIS_STARTUP_RECOVERY === 'true') {
    assert.deepEqual(await provider('Definition', document, position(source, source.indexOf('value;'))), []);
    assert.deepEqual(await provider('Hover', document, position(source, source.indexOf('value;'))), []);
    await vscode.workspace.getConfiguration('iris').update('serverPath', process.env.IRIS_STARTUP_SERVER, vscode.ConfigurationTarget.Global);
    let timer;
    try {
      await Promise.race([
        vscode.commands.executeCommand('iris.restartLanguageServer'),
        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('Restart timed out')), 30000); }),
      ]);
    } finally {
      clearTimeout(timer);
    }
  }

  await eventually(`${suffix} definition and Hover availability`, async () => {
    assert.deepEqual(locations(await provider('Definition', document, position(source, source.indexOf('value;')))), locations([
      location(document.uri, range(source, 'value')),
    ]));
    await assertHover(document, range(source, 'value', source.indexOf('value;')), [/\bvalue\b/, /\bInteger\b/]);
  });
  if (process.env.IRIS_STARTUP_RECOVERY === 'true') console.log('PASS: missing executable recovered after User serverPath correction and explicit native restart command');
  assert.deepEqual(locations(await provider('Reference', document, position(source, source.indexOf('value')))), locations([
    location(document.uri, range(source, 'value')),
    location(document.uri, range(source, 'value', source.indexOf('value;'))),
  ]));
  const completions = await provider('CompletionItem', document, position(source, source.indexOf('value;') + 2));
  assert.ok(completions.items.some(item => item.label === 'value'));
  assert.ok((await hints(document)).some(hint => hint.label === ': Integer'));
  const edits = await provider('FormatDocument', document, { tabSize: 2, insertSpaces: true });
  assert.ok(edits.length > 0);
  const workspaceEdit = new vscode.WorkspaceEdit();
  workspaceEdit.set(document.uri, edits);
  assert.equal(await vscode.workspace.applyEdit(workspaceEdit), true);
  assert.ok(document.getText().includes('let value = 1'));
  console.log(`PASS: actual ${suffix} autoactivation, exact shadow-safe references, definitions, Hover, completion, inlay hints and applied formatting`);

  const members = 'class Box { public fun read() {} private fun secret() {} public class fun build() {} } module Main { let item = Box.new(); item.re }';
  await replace(editor, members);
  await eventually('.ir source-defined member completion', async () => {
    const result = await provider('CompletionItem', document, position(members, members.lastIndexOf('re }') + 2));
    assert.deepEqual(result.items.map(item => item.label), ['read']);
  });
  const irisFile = path.join(process.env.IRIS_STARTUP_WORKSPACE, 'unchanged.iris');
  await fs.writeFile(irisFile, source);
  const irisDocument = await vscode.workspace.openTextDocument(irisFile);
  await vscode.window.showTextDocument(irisDocument);
  assert.equal(irisDocument.languageId, 'iris');
  await eventually('.iris provider preservation', async () => {
    assert.deepEqual(locations(await provider('Definition', irisDocument, position(source, source.indexOf('value;')))), locations([
      location(irisDocument.uri, range(source, 'value')),
    ]));
    assert.ok((await provider('FormatDocument', irisDocument, { tabSize: 2, insertSpaces: true })).length > 0);
    await assertHover(irisDocument, range(source, 'value', source.indexOf('value;')), [/\bvalue\b/, /\bInteger\b/]);
  });
  console.log('PASS: source-defined .ir member completion and unchanged .iris recognition/providers');
};
