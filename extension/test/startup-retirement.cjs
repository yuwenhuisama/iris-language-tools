const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const vscode = require('vscode');
const { eventually, provider, position, locations, location, range } = require('./semantic-support.cjs');
const { assertHover } = require('./hover-integration.cjs');

async function bounded(promise, milliseconds = 10000) {
  let timer;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error('Lifecycle operation timed out')), milliseconds);
    })]);
  } finally {
    clearTimeout(timer);
  }
}

exports.run = async function () {
  const fixture = process.env.IRIS_STARTUP_FIXTURE;
  const marker = `${fixture}.initialized`;
  const source = 'module Main { let value=1; value; }';
  const file = path.join(process.env.IRIS_STARTUP_WORKSPACE, 'pending.ir');
  await fs.writeFile(file, source);
  const events = fs.watch(path.dirname(marker), { signal: AbortSignal.timeout(10000) });
  let pid;
  try {
    const initialized = (async () => {
      for await (const event of events) {
        if (event.filename === path.basename(marker)) {
          const record = JSON.parse(await fs.readFile(marker, 'utf8'));
          assert.equal(Number.isInteger(record.pid), true);
          return record.pid;
        }
      }
      assert.fail('Fixture never received initialize');
    })();
    const document = await vscode.workspace.openTextDocument(file);
    await vscode.window.showTextDocument(document);
    pid = await initialized;
    const extension = require('../dist/extension.cjs');
    const mode = process.env.IRIS_STARTUP_MODE;
    if (mode === 'timeout') {
      const hostExtension = vscode.extensions.getExtension('iris-local.iris-language-tools');
      await bounded(hostExtension.activate(), 55000);
      assert.throws(() => process.kill(pid, 0), { code: 'ESRCH' });
    }
    if (mode === 'deactivate') {
      await bounded(extension.deactivate());
      await eventually('retired process exits', async () => assert.throws(() => process.kill(pid, 0), { code: 'ESRCH' }));
      assert.deepEqual(await provider('Definition', document, position(source, source.indexOf('value;'))), []);
      assert.deepEqual(await provider('Hover', document, position(source, source.indexOf('value;'))), []);
      console.log('PASS: unresolved real initialize deactivated, child exited, providers absent');
      await bounded(require('./retired-transport.cjs').checkRetiredTransport());
      return;
    }
    await vscode.workspace.getConfiguration('iris').update('serverPath', process.env.IRIS_STARTUP_SERVER, vscode.ConfigurationTarget.Global);
    await bounded(Promise.all(Array.from({ length: 3 }, () => vscode.commands.executeCommand('iris.restartLanguageServer'))));
    await eventually('retired process exits', async () => assert.throws(() => process.kill(pid, 0), { code: 'ESRCH' }));
    await eventually('replacement definition provider', async () => {
      assert.deepEqual(locations(await provider('Definition', document, position(source, source.indexOf('value;')))), locations([
        location(document.uri, range(source, 'value')),
      ]));
      await assertHover(document, range(source, 'value', source.indexOf('value;')), [/\bvalue\b/, /\bInteger\b/]);
    });
    await bounded(extension.deactivate());
    console.log(`PASS: real nonresponding initialize ${mode}, child retired, corrected concurrent restart restored exact definitions`);
  } finally {
    await events.return();
    if (pid !== undefined) {
      try { process.kill(pid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
    }
  }
};
