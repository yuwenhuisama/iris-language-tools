const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const vscode = require('vscode');

exports.verifyRuntimeFormatting = async function () {
  const executable = process.env.IRIS_TEST_EXECUTABLE;
  if (!executable) {
    console.log('SKIP: VM formatting equivalence; set IRIS_TEST_EXECUTABLE to an existing Iris CLI');
    return;
  }
  const fixtures = [
    { source: 'if true {\nprint(1 + 2)\n}\n', expected: '3\n' },
    { source: 'mut n = 0\nwhile n < 3 {\nprint(n)\nn = n + 1\n}\n', expected: '0\n1\n2\n' },
  ];
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'iris runtime format '));
  try {
    for (const [index, fixture] of fixtures.entries()) {
      const document = await vscode.workspace.openTextDocument({ language: 'iris', content: fixture.source });
      const edits = await vscode.commands.executeCommand('vscode.executeFormatDocumentProvider', document.uri, { tabSize: 4, insertSpaces: true });
      assert.ok(edits?.length > 0);
      const changes = new vscode.WorkspaceEdit();
      changes.set(document.uri, edits);
      assert.equal(await vscode.workspace.applyEdit(changes), true);
      const original = path.join(root, `original ${index}.iris`);
      const formatted = path.join(root, `formatted ${index}.iris`);
      await fs.writeFile(original, fixture.source);
      await fs.writeFile(formatted, document.getText());
      for (const file of [original, formatted]) {
        const result = spawnSync(executable, ['--vm', file], { cwd: root, encoding: 'utf8', timeout: 10000 });
        assert.ifError(result.error);
        assert.equal(result.status, 0, result.stderr);
        assert.equal(result.stdout.replaceAll('\r\n', '\n'), fixture.expected);
        assert.equal(result.stderr, '');
      }
    }
    console.log('PASS: original and formatted arithmetic/loop programs have identical expected VM output');
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
};
