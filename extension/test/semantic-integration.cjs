const assert = require('node:assert/strict');
const vscode = require('vscode');
const { eventually, provider, position, range, coordinates, locations, location, hints, hint, replace } = require('./semantic-support.cjs');

exports.verifySemantics = async function () {
  const source = "module Main { let text = '😀'; let value = 1; value; fun read(value) { value } }";
  const document = await vscode.workspace.openTextDocument({ language: 'iris', content: source });
  const editor = await vscode.window.showTextDocument(document);
  try {
    assert.equal(vscode.workspace.getConfiguration('editor', document).get('inlayHints.enabled'), 'on');
    await eventually('UTF-16 local definition in unsaved buffer', async () => {
      const actual = await provider('Definition', document, position(source, source.indexOf('value;')));
      assert.deepEqual(locations(actual), locations([location(document.uri, range(source, 'value'))]));
    });
    const references = await provider('Reference', document, position(source, source.indexOf('value')));
    assert.deepEqual(locations(references), locations([
      location(document.uri, range(source, 'value')),
      location(document.uri, range(source, 'value', source.indexOf('value;'))),
    ]), 'References include the declaration and outer use, never the shadowing parameter or its use');
    const innerDefinition = await provider('Definition', document, position(source, source.lastIndexOf('value')));
    assert.deepEqual(locations(innerDefinition), locations([
      location(document.uri, range(source, 'value', source.indexOf('value)'))),
    ]));
    const symbols = await provider('CompletionItem', document, position(source, source.indexOf('value;') + 2));
    const value = symbols.items.filter(item => item.label === 'value');
    assert.equal(value.length, 1);
    assert.equal(value[0].kind, vscode.CompletionItemKind.Variable);
    assert.deepEqual(coordinates(value[0].range), coordinates(range(source, 'value', source.indexOf('value;'))));
    const localHints = await hints(document);
    assert.ok(localHints.some(value => JSON.stringify(value) === JSON.stringify(hint(source, source.indexOf('value') + 5, ': Integer'))));
    console.log('PASS: real definition, shadow-safe references, symbol completion and Integer hints with UTF-16 ranges');

    const methods = 'module Main { fun read(value = 1) { 2 }; fun typed(value: Integer) -> Integer { value }; let local = 1; let explicit: Integer = 2 }';
    await replace(editor, methods);
    await eventually('omitted versus explicit type annotations', async () => {
      assert.deepEqual(await hints(document), [
        hint(methods, methods.indexOf('value') + 5, ': Dynamic<Object>'),
        hint(methods, methods.indexOf(')') + 1, ' -> Dynamic<Object>'),
        hint(methods, methods.indexOf('local') + 5, ': Integer'),
      ], 'Only omitted method annotations and inferred locals receive hints');
    });
    console.log('PASS: omitted parameter/return stay Dynamic<Object>; explicit annotations have no redundant hints');

    const members = 'class Box { public fun read() {} private fun secret() {} public class fun build() {} } module Main { let item = Box.new(); item.re }';
    await replace(editor, members);
    await eventually('known instance member completion', async () => {
      const result = await provider('CompletionItem', document, position(members, members.lastIndexOf('re }') + 2));
      assert.deepEqual(result.items.map(item => item.label), ['read']);
      assert.equal(result.items[0].kind, vscode.CompletionItemKind.Method);
      assert.deepEqual(coordinates(result.items[0].range), coordinates(range(members, 're', members.lastIndexOf('re }'))));
    });
    const trailing = members.slice(0, members.lastIndexOf('re }'));
    await replace(editor, trailing);
    await eventually('trailing-dot completion during incomplete edit', async () => {
      const result = await provider('CompletionItem', document, position(trailing, trailing.length));
      assert.equal(result.isIncomplete, true);
      assert.deepEqual(result.items.map(item => item.label), ['read']);
      assert.deepEqual(coordinates(result.items[0].range), coordinates(range(trailing, '', trailing.length)));
    });
    assert.equal(document.isDirty, true);
    console.log('PASS: known instance completion excludes private/class members and survives an unsaved trailing dot');
  } finally {
    await vscode.window.showTextDocument(document);
    await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
  }
  if (process.env.IRIS_TEST_EMPTY !== '1') await require('./semantic-workspace.cjs').verifyWorkspace();
};
