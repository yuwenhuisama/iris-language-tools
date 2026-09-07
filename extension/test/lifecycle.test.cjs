const assert = require('node:assert/strict');
const { test } = require('node:test');
const manifest = require('../package.json');
const { scenario, deferred } = require('./lifecycle-support.cjs');

test('recognizes both v1 suffixes when the language contribution is loaded', () => {
  const language = manifest.contributes.languages.find(value => value.id === 'iris');
  assert.deepEqual(language.extensions.slice().sort(), ['.ir', '.iris']);
});

test('preserves machine scope and trust when restart is contributed', () => {
  assert.equal(manifest.capabilities.untrustedWorkspaces.supported, false);
  assert.equal(manifest.contributes.configuration.properties['iris.serverPath'].scope, 'machine');
  assert.equal(manifest.contributes.commands.find(value => value.command === 'iris.restartLanguageServer')?.enablement, 'isWorkspaceTrusted');
});

test('recovers when a missing executable is corrected and explicitly restarted', async () => {
  const input = scenario({ failedDispose: true });
  await input.activate(input.context);
  assert.equal(input.errors.length, 1);
  assert.ok(input.commands.has('iris.runFile'));
  input.state.command = '/correct/server';
  assert.ok(input.commands.has('iris.restartLanguageServer'));
  await input.commands.get('iris.restartLanguageServer')();
  assert.equal(input.clients.filter(value => value.running).length, 1);
  assert.equal(input.clients.at(-1).command, '/correct/server');
  assert.equal(input.clients[0].disposed, true);
  assert.deepEqual(input.watchers.filter(value => !value.disposed).map(value => value.pattern).sort(), ['**/*.ir', '**/*.iris', '**/iris.toml']);
  await input.deactivate();
  assert.ok(input.watchers.every(value => value.disposed));
});

test('coalesces concurrent restarts and disposes the previous client before construction', async () => {
  const input = scenario({ command: 'first' });
  await input.activate(input.context);
  input.state.command = 'second';
  input.state.disposeGate = deferred();
  assert.ok(input.commands.has('iris.restartLanguageServer'));
  const restart = input.commands.get('iris.restartLanguageServer');
  const pending = [restart(), restart(), restart()];
  await Promise.resolve();
  assert.equal(input.clients.length, 1);
  input.state.disposeGate.resolve();
  await Promise.all(pending);
  assert.deepEqual(input.events, ['construct:first', 'start:first', 'dispose:first', 'construct:second', 'start:second']);
  assert.equal(input.clients.filter(value => value.running).length, 1);
  await input.deactivate();
  assert.ok(input.watchers.every(value => value.disposed));
});

test('cleans up and refuses pending restarts when deactivation races startup', async () => {
  const started = deferred();
  const gate = deferred();
  const input = scenario({ command: 'server', started, startGate: gate });
  const activation = input.activate(input.context);
  await started.promise;
  const closing = input.deactivate();
  const finished = Promise.all([activation, closing]);
  assert.equal(await settlesThisTurn(finished), true, 'deactivation must not wait for initialize');
  assert.equal(input.clients.filter(value => !value.disposed).length, 0);
  assert.ok(input.watchers.every(value => value.disposed));
  await input.commands.get('iris.restartLanguageServer')?.();
  assert.equal(input.clients.length, 1);
  gate.resolve();
  await new Promise(setImmediate);
  assert.equal(input.clients[0].running, false, 'late initialize must not restore retired providers');
});

async function settlesThisTurn(promise) {
  return Promise.race([promise.then(() => true), new Promise(resolve => setImmediate(() => resolve(false)))]);
}

test('retires unresolved initialization when the 45-second deadline expires', async () => {
  const started = deferred();
  const input = scenario({ command: 'nonresponsive-server', started, startGate: deferred() });
  const activation = input.activate(input.context);
  await started.promise;
  input.expireStartup();
  assert.equal(await settlesThisTurn(activation), true, 'startup timeout must settle activation');
  assert.ok(input.watchers.every(watcher => watcher.disposed));
  assert.equal(input.clients[0].disposed, true);
  assert.equal(input.errors.length, 1);
  input.state.command = '/correct/server';
  input.state.startGate = undefined;
  await input.commands.get('iris.restartLanguageServer')();
  assert.equal(input.clients.at(-1).running, true);
  await input.deactivate();
  assert.equal(input.timers.size, 0);
});

test('interrupts unresolved startup and coalesces corrected concurrent restarts', async () => {
  const started = deferred();
  const oldGate = deferred();
  const input = scenario({ command: 'nonresponsive-server', started, startGate: oldGate });
  const activation = input.activate(input.context);
  await started.promise;
  input.state.command = '/correct/server';
  input.state.startGate = undefined;
  const restart = input.commands.get('iris.restartLanguageServer');
  const replacement = restart();
  assert.equal(restart(), replacement);
  assert.equal(await settlesThisTurn(Promise.all([activation, replacement])), true, 'restart must interrupt initialize');
  oldGate.resolve();
  await new Promise(setImmediate);
  assert.deepEqual(input.clients.filter(client => client.running).map(client => client.command), ['/correct/server']);
  assert.equal(input.clients[0].disposed, true);
  assert.equal(input.watchers.filter(watcher => !watcher.disposed).length, 3);
  assert.equal(input.errors.length, 0, 'explicit interruption must not offer stale recovery');
  await input.deactivate();
});

test('refuses corrected replacement when trust is revoked during unresolved startup retirement', async () => {
  const started = deferred();
  const input = scenario({ command: 'nonresponsive-server', started, startGate: deferred() });
  const activation = input.activate(input.context);
  await started.promise;
  input.state.command = '/correct/server';
  const replacement = input.commands.get('iris.restartLanguageServer')();
  input.state.trusted = false;
  assert.equal(await settlesThisTurn(Promise.all([activation, replacement])), true);
  assert.equal(input.clients.length, 1);
  assert.ok(input.watchers.every(watcher => watcher.disposed));
  await input.deactivate();
});

test('handles construction failure without losing the run or restart commands', async () => {
  const input = scenario({ constructionFailure: true });
  await input.activate(input.context);
  assert.equal(input.errors.length, 1);
  assert.ok(input.commands.has('iris.runFile'));
  assert.ok(input.commands.has('iris.restartLanguageServer'));
  assert.ok(input.watchers.every(value => value.disposed));
  await input.deactivate();
});

test('refuses all startup when the workspace is untrusted', async () => {
  const input = scenario({ trusted: false });
  await input.activate(input.context);
  assert.equal(input.clients.length, 0);
  assert.equal(input.watchers.length, 0);
  assert.equal(input.commands.size, 0);
});

test('checks trust again when restart is invoked', async () => {
  const input = scenario({ command: 'server' });
  await input.activate(input.context);
  input.state.trusted = false;
  assert.ok(input.commands.has('iris.restartLanguageServer'));
  await input.commands.get('iris.restartLanguageServer')();
  assert.equal(input.clients.length, 1);
  await input.deactivate();
});

test('refuses replacement when trust changes while the previous client is disposing', async () => {
  const input = scenario({ command: 'server' });
  await input.activate(input.context);
  input.state.disposeGate = deferred();
  const restart = input.commands.get('iris.restartLanguageServer')();
  input.state.trusted = false;
  input.state.disposeGate.resolve();
  await restart;
  assert.equal(input.clients.length, 1);
  assert.equal(input.clients[0].running, false);
  assert.ok(input.watchers.every(value => value.disposed));
  await input.deactivate();
});

test('prevents replacement when deactivation interrupts a queued restart', async () => {
  const input = scenario({ command: 'server' });
  await input.activate(input.context);
  input.state.disposeGate = deferred();
  const restart = input.commands.get('iris.restartLanguageServer')();
  const closing = input.deactivate();
  input.state.disposeGate.resolve();
  await Promise.all([restart, closing]);
  assert.equal(input.clients.length, 1);
  assert.equal(input.clients[0].running, false);
  assert.ok(input.watchers.every(value => value.disposed));
});

test('reports command and negotiated capabilities when normal startup succeeds', async () => {
  const input = scenario({ command: '/selected/server' });
  await input.activate(input.context);
  const records = input.lines.map(line => JSON.parse(line.slice(line.indexOf('{'))));
  assert.deepEqual(records, [
    { command: '/selected/server' },
    { serverInfo: { name: 'fixture-server', version: '1' }, capabilities: {
      formatting: true, definition: true, references: true, completion: true, inlayHints: true, hover: true,
    } },
  ]);
  assert.equal(input.warnings.length, 0);
  await input.deactivate();
});

test('retains available features when the server advertises only completion', async () => {
  const input = scenario({ command: 'old-server', capabilities: { completionProvider: {} } });
  await input.activate(input.context);
  assert.equal(input.warnings.length, 1);
  assert.equal(input.clients[0].running, true);
  const initialized = JSON.parse(input.lines[1].slice(input.lines[1].indexOf('{')));
  assert.deepEqual(initialized.capabilities, {
    formatting: false, definition: false, references: false, completion: true, inlayHints: false, hover: false,
  });
  assert.ok(input.lines.length >= 2);
  await input.deactivate();
});
