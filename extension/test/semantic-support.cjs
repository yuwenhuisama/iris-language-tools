const assert = require('node:assert/strict');
const { setTimeout: delay } = require('node:timers/promises');
const vscode = require('vscode');

async function eventually(label, check) {
  const deadline = Date.now() + 20000;
  let lastFailure;
  while (Date.now() < deadline) {
    try {
      return await check();
    } catch (error) {
      if (!(error instanceof assert.AssertionError)) throw error;
      lastFailure = error;
    }
    await delay(100);
  }
  throw new Error(`Timed out: ${label}\n${lastFailure?.message}`, { cause: lastFailure });
}

async function provider(name, document, argument) {
  let timer;
  try {
    return await Promise.race([
      vscode.commands.executeCommand(`vscode.execute${name}Provider`, document.uri, argument),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new Error(`${name} provider timed out`)), 15000);
      }),
    ]);
  } finally {
    clearTimeout(timer);
  }
}

function position(source, offset) {
  const lines = source.slice(0, offset).split('\n');
  return new vscode.Position(lines.length - 1, lines.at(-1).length);
}

function range(source, name, offset = source.indexOf(name)) {
  assert.ok(offset >= 0, `Missing fixture token ${name}`);
  return new vscode.Range(position(source, offset), position(source, offset + name.length));
}

function coordinates(value) {
  return [value.start.line, value.start.character, value.end.line, value.end.character];
}

function locations(values) {
  return (values ?? []).map(value => ({ uri: value.uri.toString(), range: coordinates(value.range) }))
    .sort((left, right) => JSON.stringify(left).localeCompare(JSON.stringify(right)));
}

function location(uri, selection) {
  return { uri, range: selection };
}

async function hints(document) {
  const values = await provider('InlayHint', document, new vscode.Range(new vscode.Position(0, 0), document.positionAt(document.getText().length)));
  return (values ?? []).map(value => ({
    position: [value.position.line, value.position.character],
    label: typeof value.label === 'string' ? value.label : value.label.map(part => part.value).join(''),
    kind: value.kind,
  }));
}

function hint(source, offset, label) {
  const anchor = position(source, offset);
  return { position: [anchor.line, anchor.character], label, kind: vscode.InlayHintKind.Type };
}

async function replace(editor, source) {
  const document = editor.document;
  assert.equal(await editor.edit(edit => edit.replace(
    new vscode.Range(document.positionAt(0), document.positionAt(document.getText().length)), source,
  )), true);
}

module.exports = { eventually, provider, position, range, coordinates, locations, location, hints, hint, replace };
