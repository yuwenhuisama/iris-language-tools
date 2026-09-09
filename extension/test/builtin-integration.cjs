const assert = require('node:assert/strict');
const { randomUUID } = require('node:crypto');
const vscode = require('vscode');
const { eventually, provider, position, range, replace } = require('./semantic-support.cjs');
const { assertHover } = require('./hover-integration.cjs');
const { assertSignature } = require('./signature-integration.cjs');

async function assertCompletion(document, cursor, expected) {
  const result = await provider('CompletionItem', document, cursor);
  assert.ok(result, 'The native completion provider must return a CompletionList');
  const labels = result.items.map(item => typeof item.label === 'string' ? item.label : item.label.label);
  for (const name of expected.present) {
    assert.ok(labels.includes(name), `Expected ${name}; received ${labels.join(', ')}`);
    assert.equal(result.items[labels.indexOf(name)].kind, vscode.CompletionItemKind.Method);
  }
  for (const name of expected.absent) {
    assert.ok(!labels.includes(name), `Unexpected ${name}; received ${labels.join(', ')}`);
  }
  return labels;
}

exports.verifyBuiltins = async function () {
  for (const suffix of ['.iris', '.ir']) {
    const uri = vscode.Uri.parse(`untitled:builtin-${randomUUID()}${suffix}`);
    const document = await vscode.workspace.openTextDocument(uri);
    const editor = await vscode.window.showTextDocument(document);
    try {
      assert.equal(document.languageId, 'iris');
      assert.equal(document.isUntitled, true);

      for (const [setup, receiver] of [
        ['', "'abc'"],
        ["let text = 'abc'; ", 'text'],
      ]) {
        const source = `module Main { ${setup}${receiver}.`;
        await replace(editor, source);
        await eventually(`${suffix} String completion when receiver is ${receiver}`, () => assertCompletion(
          document, document.positionAt(source.length), { present: ['replace', 'trim'], absent: ['push'] },
        ));
      }
      console.log(`PASS: ${suffix} untitled native provider acceptance: literal and binding String completion includes replace/trim, excludes push`);

      const call = "module Main {\r\n '\u{1f600}'.replace('a', 'b')\r\n}";
      await replace(editor, call);
      await eventually(`${suffix} builtin Hover when querying replace after a UTF-16 surrogate pair`, () => assertHover(
        document, range(call, 'replace'), [/\breplace\s*\(/, /\bString\b/],
      ));
      await eventually(`${suffix} replace SignatureHelp when cursor is in the second slot`, async () => {
        const help = await assertSignature(document, position(call, call.indexOf("'b'")), {
          active: 1, label: /\breplace\(.*\)\s*->\s*String\b/, parameter: /:\s*String\b/,
        });
        assert.equal(help.signatures[0].parameters.length, 2);
      });
      console.log(`PASS: ${suffix} untitled native provider acceptance: replace Hover has String and exact UTF-16 name range; SignatureHelp selects slot 2`);

      for (const [receiver, present, absent] of [
        ['[1, 2]', ['length', 'push'], ['size', 'share_count', 'replace']],
        ['Float64', ['from_bits'], ['to_bits']],
        ['(1.0)', ['to_bits'], ['from_bits']],
      ]) {
        const source = `module Main { ${receiver}.`;
        await replace(editor, source);
        await eventually(`${suffix} builtin completion when receiver is ${receiver}`, () => assertCompletion(
          document, document.positionAt(source.length), { present, absent },
        ));
      }
      console.log(`PASS: ${suffix} untitled native provider acceptance: Array excludes size/share_count; Float64 class and literal methods stay distinct`);

      const local = 'class String { public fun source_only() {} public fun replace(value: Integer) -> Integer { value } }\n'
        + 'module Main { let text: String = String.new(); text.replace(1) }';
      await replace(editor, local);
      const localName = local.lastIndexOf('replace');
      await eventually(`${suffix} source methods when a local class is named String`, async () => {
        await assertCompletion(document, position(local, localName), {
          present: ['source_only', 'replace'], absent: ['trim', 'push'],
        });
        await assertHover(document, range(local, 'replace', localName), [
          /\breplace\(value:\s*Integer\)/, /->\s*Integer\b/,
        ]);
        const help = await assertSignature(document, position(local, localName + 'replace('.length), {
          active: 0, label: /\breplace\(value:\s*Integer\)\s*->\s*Integer\b/, parameter: /^value:\s*Integer$/,
        });
        assert.equal(help.signatures[0].parameters.length, 1);
      });
      const shadowedLiteral = local.slice(0, local.indexOf('module Main')) + "module Main { 'abc'.";
      await replace(editor, shadowedLiteral);
      await eventually(`${suffix} literal identity when source declares a String class`, () => assertCompletion(
        document, document.positionAt(shadowedLiteral.length), {
          present: ['replace', 'trim'], absent: ['source_only', 'push'],
        },
      ));
      console.log(`PASS: ${suffix} untitled native provider acceptance: local String methods retain source completion/Hover/signature; literals retain builtin identity`);

      const dynamic = "module Main { 'abc'.replace('a', 'b'); let text: Dynamic<String> = 'abc'; text.replace('a', 'b') }";
      await replace(editor, dynamic);
      const dynamicName = dynamic.lastIndexOf('replace');
      await eventually(`${suffix} empty builtin results when binding is annotated Dynamic<String>`, async () => {
        await assertHover(document, range(dynamic, 'replace'), [/\breplace\s*\(/, /\bString\b/]);
        await assertCompletion(document, position(dynamic, dynamicName), {
          present: [], absent: ['replace', 'trim', 'push'],
        });
        const completion = await provider('CompletionItem', document, position(dynamic, dynamicName));
        assert.ok(completion.items.every(item => item.kind !== vscode.CompletionItemKind.Method),
          'Dynamic<String> must not expose methods; editor word suggestions are independent');
        assert.deepEqual(await provider('Hover', document, position(dynamic, dynamicName)), []);
        const help = await provider('SignatureHelp', document, position(dynamic, dynamic.lastIndexOf("'b'")));
        assert.ok(!help || help.signatures.length === 0, 'Dynamic<String> must not borrow the String signature');
      });
      console.log(`PASS: ${suffix} untitled native provider acceptance: Dynamic<String> has no method completion, Hover or SignatureHelp; editor word suggestions are independent`);

      const original = "module Main { let text = 'abc'; text.";
      await replace(editor, original);
      await eventually(`${suffix} String methods before an unsaved receiver edit`, () => assertCompletion(
        document, document.positionAt(original.length), { present: ['replace', 'trim'], absent: ['push'] },
      ));
      const version = document.version;
      assert.equal(await editor.edit(edit => edit.replace(range(original, "'abc'"), '[1, 2]')), true);
      await eventually(`${suffix} Array methods when the same binding initializer changes unsaved`, () => assertCompletion(
        document, document.positionAt(document.getText().length), {
          present: ['length', 'push'], absent: ['replace', 'trim', 'size', 'share_count'],
        },
      ));
      assert.ok(document.version > version);
      assert.equal(document.isDirty, true);
      console.log(`PASS: ${suffix} untitled native provider acceptance: unsaved String-to-Array receiver edit updates methods (no physical typing or widget-pixel claim)`);

      const classCopy = 'module Main { let klass = Float64; klass.';
      await replace(editor, classCopy);
      await eventually(`${suffix} class identity survives an immutable copy`, () => assertCompletion(
        document, document.positionAt(classCopy.length), {
          present: ['from_bits', 'define_method'], absent: ['to_bits'],
        },
      ));
      const methodReads = 'module Main { Object.new().hash().div(1); Object.new().hash.div(1) }';
      await replace(editor, methodReads);
      await eventually(`${suffix} Object method reads are not invocation results`, async () => {
        await assertSignature(document, position(methodReads, methodReads.indexOf('div(1)') + 4), {
          active: 0, label: /\bdiv\(/, parameter: /Integer/,
        });
        const result = await provider('SignatureHelp', document,
          position(methodReads, methodReads.lastIndexOf('div(1)') + 4));
        assert.ok(!result || result.signatures.length === 0,
          'Reading Object.hash must not infer the Integer returned by calling it');
      });
      console.log(`PASS: ${suffix} class-value copies retain metadata; Object method reads do not acquire return-type signatures`);
    } finally {
      await vscode.window.showTextDocument(document);
      await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
    }
  }
};
