const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { test } = require('node:test');
const manifest = require('../package.json');
const configuration = require('../language-configuration.json');

function enterAction(beforeText, afterText = '', previousLineText = '') {
  return configuration.onEnterRules.find(rule =>
    new RegExp(rule.beforeText).test(beforeText)
    && (rule.afterText === undefined || new RegExp(rule.afterText).test(afterText))
    && (rule.previousLineText === undefined || new RegExp(rule.previousLineText).test(previousLineText)))?.action.indent;
}

function snippets() {
  const contribution = manifest.contributes.snippets.find(value => value.language === 'iris');
  assert.equal(contribution.path, './snippets/iris.json');
  return JSON.parse(fs.readFileSync(path.resolve(__dirname, '..', contribution.path), 'utf8'));
}

function expand(body) {
  return body.join('\n').replace(/\$\{\d+:([^}]*)\}/g, '$1').replace(/\$\d+/g, '');
}

if (process.env.IRIS_EDITING_HOST !== '1') {
  test('uses overridable Iris-only spaces when language defaults are contributed', () => {
    const defaults = manifest.contributes.configurationDefaults;
    assert.deepEqual(defaults, { '[iris]': { 'editor.tabSize': 2, 'editor.insertSpaces': true, 'editor.inlayHints.enabled': 'on' } });
  });

  for (const [before, after] of [
    ['if true {', '}'], ['class Example {', '}'], ['let values = %{', '}'],
    ['send(', ')'], ['let values = [', ']'], ['    %{  ', '  },'],
  ]) {
    test(`indents both lines when Enter splits ${JSON.stringify(before + after)}`, () => {
      const action = enterAction(before, after);
      assert.equal(action, 'indentOutdent');
    });
    test(`increases indentation when a safe line ends with ${JSON.stringify(before)}`, () => {
      const matches = new RegExp(configuration.indentationRules.increaseIndentPattern).test(before);
      assert.equal(matches, true);
      assert.equal(enterAction(before), 'indent');
    });
  }

  for (const line of ['}', '    )', '\t],', '});', '])']) {
    test(`decreases indentation when the line contains only closers ${JSON.stringify(line)}`, () => {
      const matches = new RegExp(configuration.indentationRules.decreaseIndentPattern).test(line);
      assert.equal(matches, true);
    });
  }

  for (const line of [
    'let text = "{', "let text = '{", 'let text = r###"{', 'let text = mbr#"[',
    'let text = """{', "let text = '''(", 'let pattern = /{', 'let pattern = r/[',
    '// {', '/// [', '/* (', ' * {', '/* nested /* x */ {', '#! /usr/bin/iris {',
    'if name == "value" {', 'if value / 2 {', 'if true { // note', 'if true { /* note */',
    'let text = "${', 'let text = "\\"{', 'if true {}', 'let values = []', 'send()',
    'let value: Array<', 'value < other', '>', '>>', '}',
  ]) {
    test(`avoids increasing indentation when a line is ambiguous or complete ${JSON.stringify(line)}`, () => {
      const matches = new RegExp(configuration.indentationRules.increaseIndentPattern).test(line);
      assert.equal(matches, false);
      assert.equal(enterAction(line, '}'), 'none');
    });
  }

  for (const previous of ['let text = """', 'let text = r#"', '/* comment', ' * comment']) {
    test(`inherits indentation when the preceding line signals literal/comment context ${JSON.stringify(previous)}`, () => {
      const action = enterAction('if true {', '}', previous);
      assert.equal(action, 'none');
    });
  }

  test('inherits indentation when paired delimiters have an ambiguous suffix', () => {
    const actions = ['} // comment', '} "', ') /* comment', '] /regex/'].map(after => enterAction('if true {', after));
    assert.deepEqual(actions, ['none', 'none', 'none', 'none']);
  });

  test('inherits indentation when paired delimiters do not match', () => {
    assert.equal(enterAction('send(', ']'), 'none');
  });

  test('keeps angles out of delimiter pairs when language configuration is loaded', () => {
    for (const pairs of [configuration.brackets, configuration.surroundingPairs]) {
      assert.equal(pairs.some(pair => pair.includes('<') || pair.includes('>')), false);
    }
    assert.equal(configuration.autoClosingPairs.some(pair => pair.open === '<' || pair.close === '>'), false);
  });

  test('contributes exactly eight snippets when the manifest path is resolved', () => {
    const prefixes = Object.values(snippets()).map(snippet => snippet.prefix).sort();
    assert.deepEqual(prefixes, ['class', 'contract', 'for', 'fun', 'if', 'let', 'module', 'while']);
  });

  test('provides sequential editable fields and a final cursor when snippets expand', () => {
    for (const snippet of Object.values(snippets())) {
      const source = snippet.body.join('\n');
      const stops = [...source.matchAll(/\$\{(\d+):[^}]*\}|\$(\d+)/g)].map(match => Number(match[1] ?? match[2]));
      assert.equal(stops.at(-1), 0, snippet.prefix);
      assert.deepEqual(stops.slice(0, -1), Array.from({ length: stops.length - 1 }, (_, index) => index + 1), snippet.prefix);
      assert.ok(stops.length > 1, snippet.prefix);
      assert.equal(expand(snippet.body).includes('$'), false, snippet.prefix);
    }
  });

  test('expands to v1 declarations and nil executable bodies when defaults are accepted', () => {
    const forms = {
      let: /^let [A-Za-z_]\w* = nil\s*$/,
      fun: /^fun [a-z_]\w*\(\) -> Nil \{\s+nil\s+\}\s*$/,
      class: /^class [A-Z]\w* \{\s+nil\s+\}\s*$/,
      module: /^module [A-Z]\w* \{\s+nil\s+\}\s*$/,
      contract: /^contract [A-Z]\w* \{\s+fun [a-z_]\w*\(\) -> Nil\s+\}\s*$/,
      if: /^if true \{\s+nil\s+\}\s*$/,
      while: /^while false \{\s+nil\s+\}\s*$/,
      for: /^for [a-z_]\w* in \[\] \{\s+nil\s+\}\s*$/,
    };
    for (const snippet of Object.values(snippets())) assert.match(expand(snippet.body), forms[snippet.prefix]);
  });
}

exports.run = async function () {
  const vscode = require('vscode');
  if (process.env.IRIS_EDITING_ENTER === '1') {
    for (const [fixture, expected] of [
      ['if true {|}', 'if true {\n  \n}'],
      ['let values = %{|}', 'let values = %{\n  \n}'],
      ['send(|)', 'send(\n  \n)'],
      ['let values = [|]', 'let values = [\n  \n]'],
      ['// {|}', '// {\n}'],
      ['let text = "{|}"', 'let text = "{\n}"'],
      ['let text = r#"{|}"#', 'let text = r#"{\n}"#'],
      ['let pattern = /{|}/', 'let pattern = /{\n}/'],
      ['/* {|} */', '/* {\n} */'],
    ]) {
      const offset = fixture.indexOf('|');
      const document = await vscode.workspace.openTextDocument({ language: 'iris', content: fixture.replace('|', '') });
      const editor = await vscode.window.showTextDocument(document);
      editor.selection = new vscode.Selection(document.positionAt(offset), document.positionAt(offset));
      await vscode.commands.executeCommand('workbench.action.focusActiveEditorGroup');
      await new Promise((resolve, reject) => {
        const timeout = setTimeout(() => { subscription.dispose(); reject(new Error(`Enter did not edit ${fixture}`)); }, 10000);
        const subscription = vscode.workspace.onDidChangeTextDocument(event => {
          if (event.document !== document || event.contentChanges.length === 0) return;
          clearTimeout(timeout);
          subscription.dispose();
          resolve();
        });
        vscode.commands.executeCommand('default:type', { text: '\n' }).then(undefined, reject);
      });
      assert.equal(document.getText(), expected, fixture);
    }
  }
  for (const snippet of Object.values(snippets())) {
    const document = await vscode.workspace.openTextDocument({ language: 'iris', content: '' });
    const editor = await vscode.window.showTextDocument(document);
    assert.equal(editor.options.tabSize, 2);
    assert.equal(editor.options.insertSpaces, true);
    await editor.insertSnippet(new vscode.SnippetString(snippet.body.join('\n')));
    assert.equal(document.getText(), expand(snippet.body).replaceAll('\t', '  '), snippet.prefix);
    await vscode.commands.executeCommand('leaveSnippet');
  }
  console.log('PASS: real editor two-space defaults and all eight snippet expansions');
};
