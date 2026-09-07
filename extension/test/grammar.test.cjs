const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { test, before } = require('node:test');
const tm = require('vscode-textmate');
const onig = require('vscode-oniguruma');
let grammar;

before(async () => {
  const wasm = fs.readFileSync(require.resolve('vscode-oniguruma/release/onig.wasm'));
  await onig.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
  const registry = new tm.Registry({
    onigLib: Promise.resolve({ createOnigScanner: p => new onig.OnigScanner(p), createOnigString: s => new onig.OnigString(s) }),
    loadGrammar: async () => JSON.parse(fs.readFileSync(path.join(__dirname, '../syntaxes/iris.tmLanguage.json'), 'utf8')),
  });
  grammar = await registry.loadGrammar('source.iris');
});

function scopes(text) {
  return grammar.tokenizeLine(text).tokens.flatMap(t => t.scopes);
}

test('highlights all fifty v1 keywords without reserving historical identifiers', () => {
  const keywords = JSON.parse(fs.readFileSync(path.join(__dirname, '../../language/keywords.json'), 'utf8'));
  assert.equal(new Set(keywords).size, 50);
  for (const word of keywords) assert.ok(scopes(word).includes('keyword.control.iris'), word);
  for (const word of ['switch', 'and', 'interface', 'new', 'className', 'class名']) {
    assert.ok(!scopes(word).includes('keyword.control.iris'), word);
  }
});

test('retains nested block comment state until outer close', () => {
  const first = grammar.tokenizeLine('/* outer /* inner */');
  const second = grammar.tokenizeLine('class still comment */ let', first.ruleStack);
  assert.ok(second.tokens[0].scopes.includes('comment.block.iris'));
  assert.ok(second.tokens.at(-1).scopes.includes('keyword.control.iris'));
});

test('strings protect keywords and raw fences do not interpret escapes', () => {
  for (const text of ['"class"', "'class'", 'm"class"', 'br"class"', 'mbr#"class"#']) {
    assert.ok(scopes(text).some(s => s.startsWith('string.')), text);
    assert.ok(!scopes(text).includes('keyword.control.iris'), text);
  }
  assert.ok(!scopes('r#"\\n"#').includes('constant.character.escape.iris'));
});

test('triple strings continue across physical lines', () => {
  const first = grammar.tokenizeLine('"""start');
  const second = grammar.tokenizeLine('class inside', first.ruleStack);
  assert.ok(second.tokens[0].scopes.some(s => s.startsWith('string.')));
});

test('registers only Iris v1 files and prevents execution in untrusted workspaces', () => {
  const manifest = require('../package.json');
  assert.deepEqual(manifest.contributes.languages[0].extensions, ['.iris', '.ir']);
  assert.equal(manifest.capabilities.untrustedWorkspaces.supported, false);
  assert.equal(manifest.contributes.configuration.properties['iris.serverPath'].scope, 'machine');
});
