const assert = require('node:assert/strict');
const vscode = require('vscode');
const { eventually, provider, range, coordinates, replace } = require('./semantic-support.cjs');

async function assertHover(document, occurrence, fragments) {
  const values = await provider('Hover', document, occurrence.start);
  assert.equal(values.length, 1, 'The real server must provide one resolved Hover');
  assert.ok(values[0].range, 'Hover must select the queried occurrence');
  assert.deepEqual(coordinates(values[0].range), coordinates(occurrence));
  const text = values[0].contents.map(content => {
    if (typeof content === 'string') return content;
    return content instanceof vscode.MarkdownString
      ? content.value.replace(/\\([\x21-\x2f\x3a-\x40\x5b-\x60\x7b-\x7e])/g, '$1')
      : content.value;
  }).join('\n');
  for (const fragment of fragments) assert.match(text, fragment);
  return text;
}

exports.assertHover = assertHover;

exports.verifyHover = async function () {
  const source = "module Main { let text = '\u{1f600}'; let value = 1; value }";
  const document = await vscode.workspace.openTextDocument({ language: 'iris', content: source });
  const editor = await vscode.window.showTextDocument(document);
  try {
    const occurrence = range(source, 'value', source.lastIndexOf('value'));
    await eventually('untitled Integer Hover with exact UTF-16 occurrence range', async () => {
      await assertHover(document, occurrence, [/\bvalue\b/, /\bInteger\b/]);
    });
    console.log('PASS: real untitled Integer Hover selects the UTF-16 use site, not its declaration');

    const changed = source.replace('value = 1', "value = 'changed'");
    await replace(editor, changed);
    await eventually('Hover reads the current unsaved variable type', async () => {
      const text = await assertHover(document, range(changed, 'value', changed.lastIndexOf('value')), [/\bvalue\b/, /\bString\b/]);
      assert.doesNotMatch(text, /\bInteger\b/);
    });
    assert.equal(document.isDirty, true);
    console.log('PASS: Hover updates Integer to String from the current untitled buffer');

    const methods = 'class Box { public fun read(value: Integer) -> String { \'result\' } }\nmodule Main { let item = Box.new(); item.read(1); item.unknown() }';
    await replace(editor, methods);
    await eventually('source method Hover signature at the call site', async () => {
      await assertHover(document, range(methods, 'read', methods.lastIndexOf('read')), [
        /\bread\s*\(/, /\bvalue\s*:\s*Integer\b/, /->\s*String\b/,
      ]);
    });
    const unknown = range(methods, 'unknown');
    assert.deepEqual(await provider('Hover', document, unknown.start), [], 'Unknown members must not synthesize Hover');
    console.log('PASS: source method Hover exposes annotated signature; unknown member has no Hover');
  } finally {
    await vscode.window.showTextDocument(document);
    await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
  }
};
