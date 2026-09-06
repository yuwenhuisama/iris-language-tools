const assert = require('node:assert/strict');
const { test } = require('node:test');
const vm = require('node:vm');
const path = require('node:path');
const { buildSync } = require('esbuild');

const bundle = buildSync({ entryPoints: [path.join(__dirname, '../src/run-file.ts')], bundle: true, platform: 'node', format: 'cjs', external: ['vscode'], write: false }).outputFiles[0].text;

function scenario(overrides = {}) {
  const tasks = [];
  let saves = 0;
  const document = {
    languageId: 'iris', uri: { scheme: 'file', fsPath: '/tmp/project with spaces/hello.iris' },
    isDirty: true, save: async () => { saves++; return true; },
    ...overrides.document,
  };
  const api = {
    workspace: { isTrusted: overrides.trusted ?? true, getConfiguration: () => ({ get: () => '/tmp/iris tools/iris' }) },
    window: { activeTextEditor: { document }, showWarningMessage: async () => {} },
    TaskScope: { Workspace: 2 },
    ProcessExecution: class { constructor(command, args, options) { Object.assign(this, { command, args, options }); } },
    Task: class { constructor(definition, scope, name, source, execution) { Object.assign(this, { definition, scope, name, source, execution }); } },
    tasks: { executeTask: async task => { tasks.push(task); } },
  };
  const module = { exports: {} };
  vm.runInNewContext(bundle, { module, exports: module.exports, require: name => name === 'vscode' ? api : require(name) });
  return { run: module.exports.runFile, tasks, saves: () => saves };
}

test('runs saved content with separate executable and VM arguments containing spaces', async () => {
  const input = scenario();
  await input.run();
  assert.equal(input.saves(), 1);
  assert.equal(input.tasks.length, 1);
  const execution = input.tasks[0].execution;
  assert.equal(execution.command, '/tmp/iris tools/iris');
  assert.deepEqual(Array.from(execution.args), ['--vm', '/tmp/project with spaces/hello.iris']);
  assert.equal(execution.options.cwd, '/tmp/project with spaces');
});

test('does not execute when save is cancelled', async () => {
  const input = scenario({ document: { save: async () => false } });
  await input.run();
  assert.equal(input.tasks.length, 0);
});

test('does not save or execute in untrusted workspace', async () => {
  const input = scenario({ trusted: false });
  await input.run();
  assert.equal(input.saves(), 0);
  assert.equal(input.tasks.length, 0);
});

test('rejects untitled buffers instead of passing a fake filesystem path', async () => {
  const input = scenario({ document: { uri: { scheme: 'untitled', fsPath: 'Untitled-1' } } });
  await input.run();
  assert.equal(input.tasks.length, 0);
});

test('runs clean files without a save operation', async () => {
  const input = scenario({ document: { isDirty: false } });
  await input.run();
  assert.equal(input.saves(), 0);
  assert.equal(input.tasks.length, 1);
});
