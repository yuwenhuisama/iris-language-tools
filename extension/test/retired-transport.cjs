const assert = require('node:assert/strict');
const vm = require('node:vm');
const path = require('node:path');
const { PassThrough } = require('node:stream');
const { buildSync } = require('esbuild');
const vscode = require('vscode');
const { StreamMessageReader, StreamMessageWriter, createProtocolConnection, CloseAction, ErrorAction } = require('vscode-languageclient/node');

function deferred() {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
}

exports.checkRetiredTransport = async function () {
  const code = buildSync({ entryPoints: [path.join(__dirname, '../src/server-client.ts')], bundle: true, platform: 'node', format: 'cjs', external: ['vscode', 'vscode-languageclient/node'], write: false }).outputFiles[0].text;
  const module = { exports: {} };
  vm.runInNewContext(code, { module, exports: module.exports, require, setTimeout, clearTimeout });
  for (const stage of ['initialize-response', 'initialized-write']) {
    const received = deferred();
    const release = deferred();
    const incoming = new PassThrough();
    const outgoing = new PassThrough();
    const output = vscode.window.createOutputChannel(`Retirement ${stage}`);
    const server = createProtocolConnection(new StreamMessageReader(outgoing), new StreamMessageWriter(incoming));
    server.onRequest('initialize', async () => {
      if (stage === 'initialize-response') { received.resolve(); await release.promise; }
      return { capabilities: { definitionProvider: true } };
    });
    server.listen();
    const client = new module.exports.ServerClient('unused-transport-fixture', {
      documentSelector: [{ language: 'iris' }], outputChannel: output,
      initializationFailedHandler: () => false,
      errorHandler: { error: () => ({ action: ErrorAction.Shutdown, handled: true }), closed: () => ({ action: CloseAction.DoNotRestart, handled: true }) },
    });
    const baseTransport = Object.getPrototypeOf(Object.getPrototypeOf(client)).createMessageTransports;
    const prototype = Object.getPrototypeOf(Object.getPrototypeOf(client));
    prototype.createMessageTransports = async () => {
      const writer = new StreamMessageWriter(outgoing);
      return { reader: new StreamMessageReader(incoming), writer: {
        onError: writer.onError, onClose: writer.onClose,
        end: () => writer.end(), dispose: () => writer.dispose(),
        write: async message => {
          if (message.method === 'initialized' && stage === 'initialized-write') { received.resolve(); await release.promise; }
          await writer.write(message);
        },
      } };
    };
    let registrations = 0;
    const definition = client.getFeature('textDocument/definition');
    const register = definition.register.bind(definition);
    definition.register = (...args) => { registrations++; return register(...args); };
    const startup = client.start().then(() => assert.fail('Retired startup resolved'), error => assert.equal(error instanceof module.exports.StartupInterrupted, true));
    try {
      await received.promise;
      client.cancelStartup();
      await client.dispose();
      release.resolve();
      await startup;
      await new Promise(setImmediate);
      await new Promise(setImmediate);
      assert.equal(registrations, 0, `${stage} must not register stale providers`);
      assert.equal(client.isRunning(), false);
      console.log(`PASS: actual languageclient rejects late ${stage} without registering old definition providers`);
    } finally {
      prototype.createMessageTransports = baseTransport;
      server.dispose();
      incoming.destroy();
      outgoing.destroy();
      output.dispose();
    }
  }
};
