const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');

const caller = 'import Core as Alias\nmodule Main { let item = Alias::Thing.new(); let result = item.read(); item.re }';
const original = 'module Core {} class Core::Thing { public fun read() -> Integer { 1 } }';
const manifest = sources => `manifest_version = 1\npackage_id = "org.example.semantic"\napi_major = 1\nversion = "1.0.0"\niris_major = 1\nsources = ${JSON.stringify(sources)}\nentry_modules = []\n[permissions]\nrequired = []\noptional = []\n`;

exports.prepare = async function (root) {
  await Promise.all([
    fs.writeFile(path.join(root, 'iris.toml'), manifest(['main.iris', 'types.iris'])),
    fs.writeFile(path.join(root, 'main.iris'), caller),
    fs.writeFile(path.join(root, 'types.iris'), original),
  ]);
};

exports.verifyWorkspace = async function () {
  const vscode = require('vscode');
  const { eventually, provider, position, range, coordinates, locations, location, hints, hint, replace } = require('./semantic-support.cjs');
  const { assertHover } = require('./hover-integration.cjs');
  const root = process.env.IRIS_TEST_WORKSPACE;
  assert.ok(root, 'The runner must supply its isolated workspace');
  assert.equal(await fs.realpath(vscode.workspace.workspaceFolders[0].uri.fsPath), await fs.realpath(root));
  const source = await vscode.workspace.openTextDocument(path.join(root, 'main.iris'));
  const targetUri = vscode.Uri.file(path.join(root, 'types.iris'));
  await vscode.window.showTextDocument(source);
  const methodUse = position(caller, caller.indexOf('read'));
  const memberCursor = position(caller, caller.lastIndexOf('re }') + 2);
  const resultOffset = caller.indexOf('result') + 6;
  const definition = () => provider('Definition', source, methodUse);
  await eventually('manifest-selected cross-file definition', async () => {
    assert.deepEqual(locations(await definition()), locations([location(targetUri, range(original, 'read'))]));
  });
  const classDefinition = await provider('Definition', source, position(caller, caller.indexOf('Thing')));
  assert.deepEqual(locations(classDefinition), locations([location(targetUri, range(original, 'Thing'))]));
  const references = await provider('Reference', source, methodUse);
  assert.deepEqual(locations(references), locations([
    location(targetUri, range(original, 'read')),
    location(source.uri, range(caller, 'read')),
  ]));
  const initialHints = await hints(source);
  await assertHover(source, range(caller, 'read'), [/\bread\s*\(/, /->\s*Integer\b/]);
  assert.ok(initialHints.some(value => JSON.stringify(value) === JSON.stringify(hint(caller, resultOffset, ': Integer'))));
  const initialMembers = await provider('CompletionItem', source, memberCursor);
  assert.deepEqual(initialMembers.items.map(item => item.label), ['read']);
  console.log('PASS: manifest-selected namespace/class navigation, cross-file references, member completion and return-type hints');

  const diskEdit = `// 😀\r\n${original}`;
  await fs.writeFile(targetUri.fsPath, diskEdit);
  await eventually('watched .iris disk edit moves unopened definition', async () => {
    assert.deepEqual(locations(await definition()), locations([location(targetUri, range(diskEdit, 'read'))]));
  });
  await fs.writeFile(path.join(root, 'iris.toml'), manifest(['main.iris']));
  await eventually('watched iris.toml excludes target', async () => {
    assert.deepEqual(locations(await definition()), []);
  });
  await fs.writeFile(path.join(root, 'iris.toml'), manifest(['main.iris', 'types.iris']));
  await eventually('watched iris.toml restores target', async () => {
    assert.deepEqual(locations(await definition()), locations([location(targetUri, range(diskEdit, 'read'))]));
  });
  console.log('PASS: real .iris and iris.toml file notifications refresh unopened package sources');

  const target = await vscode.workspace.openTextDocument(targetUri);
  const editor = await vscode.window.showTextDocument(target);
  try {
    const dirtyText = "// 😀\r\nmodule Core {} class Core::Thing { public fun reset() -> Integer { 0 } public fun read() -> String { 'changed' } }";
    await replace(editor, dirtyText);
    await eventually('dirty cross-file definition, references, members and hints', async () => {
      assert.deepEqual(locations(await definition()), locations([location(targetUri, range(dirtyText, 'read'))]));
      const updatedReferences = await provider('Reference', source, methodUse);
      assert.deepEqual(locations(updatedReferences), locations([
        location(targetUri, range(dirtyText, 'read')),
        location(source.uri, range(caller, 'read')),
      ]));
      const completion = await provider('CompletionItem', source, memberCursor);
      assert.deepEqual(completion.items.map(item => item.label).sort(), ['read', 'reset']);
      for (const item of completion.items) {
        assert.deepEqual(coordinates(item.range), coordinates(range(caller, 're', caller.lastIndexOf('re }'))));
      }
      const updatedHints = await hints(source);
      const hover = await assertHover(source, range(caller, 'read'), [/\bread\s*\(/, /->\s*String\b/]);
      assert.doesNotMatch(hover, /->\s*Integer\b/);
      assert.ok(updatedHints.some(value => JSON.stringify(value) === JSON.stringify(hint(caller, resultOffset, ': String'))));
      assert.ok(!updatedHints.some(value => JSON.stringify(value) === JSON.stringify(hint(caller, resultOffset, ': Integer'))));
    });
    assert.equal(target.isDirty, true);
    assert.equal(await fs.readFile(targetUri.fsPath, 'utf8'), diskEdit, 'Semantic queries must not save or execute dirty targets');
    console.log('PASS: unsaved cross-file edits update exact definitions/references, new members, String hints and call-site Hover without saving');
  } finally {
    await vscode.window.showTextDocument(target);
    await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
    await vscode.window.showTextDocument(source);
    await vscode.commands.executeCommand('workbench.action.closeActiveEditor');
  }
};
