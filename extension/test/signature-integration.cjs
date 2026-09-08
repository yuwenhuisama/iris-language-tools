const assert = require('node:assert/strict');
const vscode = require('vscode');
const { eventually, provider, replace } = require('./semantic-support.cjs');

async function assertSignature(document, cursor, expected) {
  const help = await provider('SignatureHelp', document, cursor);
  assert.ok(help, 'The real server must resolve SignatureHelp');
  assert.equal(help.signatures.length, 1);
  assert.equal(help.activeSignature, 0);
  assert.equal(help.activeParameter, expected.active);
  const signature = help.signatures[0];
  assert.match(signature.label, expected.label);
  for (const parameter of signature.parameters) {
    assert.ok(Array.isArray(parameter.label), 'Parameter labels must use UTF-16 offsets');
    const [start, end] = parameter.label;
    assert.ok(start < end && end <= signature.label.length);
  }
  if (expected.parameter) {
    const [start, end] = signature.parameters[help.activeParameter].label;
    assert.match(signature.label.slice(start, end), expected.parameter);
  }
  if (expected.docs) {
    assert.ok(signature.documentation instanceof vscode.MarkdownString);
    assert.equal(signature.documentation.isTrusted, false);
    assert.equal(signature.documentation.supportHtml, false);
    assert.match(signature.documentation.value, expected.docs);
  }
  return help;
}

exports.assertSignature = assertSignature;

exports.verifySignatures = async function () {
  const declaration = 'module Main {\n/// Choose a value.\npublic fun choose(_, value = \'😀\', *rest, key option, **kwargs) -> String {}\npublic fun inner(item) {}\n';
  const document = await vscode.workspace.openTextDocument({ language: 'iris', content: `${declaration}choose(` });
  const editor = await vscode.window.showTextDocument(document);
  try {
    for (const [call, active, parameter, label] of [
      ['choose(', 0, /^_:/, /choose\(/],
      ['choose(1,', 1, /^value:/, /choose\(/],
      ['choose(1, 2, 3, 4', 2, /^\*rest:/, /choose\(/],
      ['choose(option:', 3, /^key option:/, /choose\(/],
      ['choose(other: 1', 4, /^\*\*kwargs:/, /choose\(/],
      ['choose(1, inner(', 0, /^item:/, /inner\(/],
      ['choose(inner(1),', 1, /^value:/, /choose\(/],
    ]) {
      await replace(editor, declaration + call);
      await eventually(`current SignatureHelp for ${call}`, () => assertSignature(
        document, document.positionAt(document.getText().length), { active, parameter, label },
      ));
    }
    await replace(editor, declaration + 'choose(1, unknown(');
    const absent = await provider('SignatureHelp', document, document.positionAt(document.getText().length));
    assert.ok(!absent || absent.signatures.length === 0, 'Unknown inner calls must not borrow outer signatures');
    console.log('PASS: real untitled SignatureHelp handles current parameters, nested calls, keyword/rest channels, discard and unknown inner calls');

  } finally {
    await vscode.window.showTextDocument(document);
    await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
  }
};

exports.verifySignatureTyping = async function () {
  const declaration = 'module Main {\n/// Choose a value.\npublic fun choose(_, value) -> String {}\n';
  const document = await vscode.workspace.openTextDocument({ language: 'iris', content: declaration + 'choose' });
  const editor = await vscode.window.showTextDocument(document, { preserveFocus: false });
  try {
    const cursor = document.positionAt(document.getText().length);
    editor.selection = new vscode.Selection(cursor, cursor);
    await vscode.commands.executeCommand('workbench.action.focusWindow');
    await vscode.commands.executeCommand('workbench.action.focusActiveEditorGroup');
    await typeInEditor(document, '(');
    assert.ok(document.getText().startsWith(declaration + 'choose('));
    await vscode.commands.executeCommand('editor.action.triggerParameterHints');
    await eventually('typed opening parenthesis SignatureHelp', () => assertSignature(document, editor.selection.active, {
      active: 0, parameter: /^_:/, label: /choose\(/, docs: /Choose a value/,
    }));
    await typeInEditor(document, '1');
    await typeInEditor(document, ',');
    assert.ok(document.getText().startsWith(declaration + 'choose(1,'));
    await vscode.commands.executeCommand('editor.action.triggerParameterHints');
    await eventually('typed comma SignatureHelp', () => assertSignature(document, editor.selection.active, {
      active: 1, parameter: /^value:/, label: /choose\(/,
    }));
    console.log('PASS: native type command changed the document for ( and ,; provider at the actual cursor confirmed active parameters 0 and 1 after triggerParameterHints (widget pixels not inspected)');
  } catch (error) {
    if (!(error instanceof NativeTypingUnavailable)) throw error;
    console.warn(`UNAVAILABLE: native SignatureHelp typing probe: ${error.message}. Direct-provider assertions remain mandatory; this is not a typing or widget pass.`);
  } finally {
    await vscode.commands.executeCommand('closeParameterHints');
    await vscode.window.showTextDocument(document);
    await vscode.commands.executeCommand('workbench.action.revertAndCloseActiveEditor');
  }
};

class NativeTypingUnavailable extends Error {}

async function typeInEditor(document, text) {
  const version = document.version;
  let timer;
  let subscription;
  try {
    const changed = new Promise((resolve, reject) => {
      timer = setTimeout(() => {
        const message = `type(${JSON.stringify(text)}) produced no matching document event; version=${version}->${document.version}, windowFocused=${vscode.window.state.focused}, activeDocumentMatches=${vscode.window.activeTextEditor?.document === document}`;
        const unfocusedHost = !vscode.window.state.focused && vscode.window.activeTextEditor?.document === document;
        reject(document.version === version && unfocusedHost ? new NativeTypingUnavailable(message) : new Error(message));
      }, 10000);
      subscription = vscode.workspace.onDidChangeTextDocument(event => {
        if (event.document === document && event.contentChanges.some(change => change.text.includes(text))) resolve();
      });
    });
    await Promise.all([vscode.commands.executeCommand('type', { text }), changed]);
  } finally {
    clearTimeout(timer);
    subscription?.dispose();
  }
}
