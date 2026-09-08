const assert = require('node:assert/strict');
const vscode = require('vscode');
const { eventually, provider, range, coordinates, replace } = require('./semantic-support.cjs');

async function assertHover(document, occurrence, fragments) {
  const values = await provider('Hover', document, occurrence.start);
  assert.equal(values.length, 1, 'The real server must provide one resolved Hover');
  assert.ok(values[0].range, 'Hover must select the queried occurrence');
  assert.deepEqual(coordinates(values[0].range), coordinates(occurrence));
  for (const content of values[0].contents) {
    if (content instanceof vscode.MarkdownString) {
      assert.equal(content.isTrusted, false);
      assert.equal(content.supportHtml, false);
      assert.match(content.value, /^`{3,}iris\n/);
    }
  }
  const text = values[0].contents.map(content => {
    if (typeof content === 'string') return content;
    return content instanceof vscode.MarkdownString
      ? content.value.replace(/\\([\x21-\x2f\x3a-\x40\x5b-\x60\x7b-\x7e])/g, '$1').replaceAll('&lt;', '<').replaceAll('&gt;', '>').replaceAll('&amp;', '&')
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

    for (const comment of [
      '/// Read safely.\n///\n/// ![image](https://example.test/a) <b> [run](command:run)\n/// @param stays plain',
      '/**\n * Read safely.\n *\n * ![image](https://example.test/a) <b> [run](command:run)\n * @param stays plain\n */',
    ]) {
      const rich = `module Core {} class Core::Box {\n${comment}\npublic fun read(value: Integer) -> String { 'result' } }\nmodule Main { let box = Core::Box.new(); box.read(1) }`;
      await replace(editor, rich);
      const occurrence = range(rich, 'read', rich.lastIndexOf('read'));
      await eventually('native rich method card with both documentation forms', async () => {
        const text = await assertHover(document, occurrence, [
          /public fun read\(value: Integer\) -> String/, /\*\*Kind:\*\* Method/,
          /\*\*Owner:\*\* Core::Box/, /\*\*Declared return:\*\* String/,
          /\*\*Documentation\*\*\n\nRead safely\.\n\n/, /@param stays plain/,
        ]);
        assert.ok(text.includes('![image](https://example.test/a) <b> [run](command:run)'));
        const raw = (await provider('Hover', document, occurrence.start))[0].contents[0].value;
        assert.ok(raw.includes('\\!\\[image\\]'));
        assert.ok(raw.includes('&lt;b&gt;'));
        assert.ok(raw.includes('\\[run\\]\\(command\\:run\\)'));
      });
      editor.selection = new vscode.Selection(occurrence.start, occurrence.start);
      await vscode.commands.executeCommand('editor.action.showHover');
      await vscode.commands.executeCommand('editor.action.hideHover');
    }
    console.log('PASS: native fenced Iris method cards show owner, declared return and separate literal line/block documentation with trusted commands and HTML disabled');
  } finally {
    await vscode.window.showTextDocument(document);
    await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
  }
};
