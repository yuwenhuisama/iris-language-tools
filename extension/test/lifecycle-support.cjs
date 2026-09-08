const vm = require('node:vm');
const path = require('node:path');
const { buildSync } = require('esbuild');

const bundle = buildSync({ entryPoints: [path.join(__dirname, '../src/extension.ts')], bundle: true, platform: 'node', format: 'cjs', external: ['vscode', 'vscode-languageclient/node'], write: false }).outputFiles[0].text;

function deferred() {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
}

function scenario(options = {}) {
  const commands = new Map();
  const clients = [];
  const watchers = [];
  const events = [];
  const errors = [];
  const warnings = [];
  const lines = [];
  const subscriptions = [];
  const timers = new Map();
  const state = { command: 'missing-server', trusted: true, ...options };
  const output = { appendLine: line => lines.push(line), show() {}, dispose() {} };
  const api = {
    workspace: {
      get isTrusted() { return state.trusted; },
      getConfiguration: () => ({ get: () => state.command }),
      createFileSystemWatcher: pattern => {
        const watcher = { pattern, disposed: false, dispose() { this.disposed = true; } };
        watchers.push(watcher);
        return watcher;
      },
    },
    commands: {
      registerCommand: (name, callback) => { commands.set(name, callback); return { dispose: () => commands.delete(name) }; },
      executeCommand: async (name, ...args) => commands.get(name)?.(...args),
    },
    window: {
      createOutputChannel: () => output,
      showErrorMessage: async (...args) => { errors.push(args); return state.action; },
      showWarningMessage: async (...args) => { warnings.push(args); },
    },
  };
  class LanguageClient {
    constructor(id, name, server, configuration) {
      if (state.constructionFailure) throw new Error('construction failed');
      Object.assign(this, { command: state.command, configuration, disposed: false, running: false });
      this.initializeResult = { serverInfo: { name: 'fixture-server', version: '1' }, capabilities: state.capabilities ?? {
        documentFormattingProvider: true, definitionProvider: true, referencesProvider: true,
        completionProvider: {}, inlayHintProvider: true, hoverProvider: true,
        signatureHelpProvider: { triggerCharacters: ['(', ','], retriggerCharacters: [','] },
      } };
      clients.push(this);
      events.push(`construct:${this.command}`);
    }
    async start() {
      if (this.startPromise) return this.startPromise;
      this.startPromise = this.beginStart();
      return this.startPromise;
    }
    async beginStart() {
      events.push(`start:${this.command}`);
      state.started?.resolve();
      await state.startGate?.promise;
      if (this.command === 'missing-server') throw new Error('ENOENT');
      if (!this.connectionClosed) this.running = true;
    }
    isRunning() { return this.running; }
    async handleConnectionClosed() { this.connectionClosed = true; this.running = false; }
    async stop() { this.running = false; }
    async dispose() {
      events.push(`dispose:${this.command}`);
      await state.disposeGate?.promise;
      this.running = false;
      this.disposed = true;
      if (state.failedDispose && this.command === 'missing-server') throw new Error('client startFailed');
    }
  }
  const module = { exports: {} };
  vm.runInNewContext(bundle, { module, exports: module.exports,
    setTimeout: (callback, milliseconds) => { const timer = {}; timers.set(timer, { callback, milliseconds }); return timer; },
    clearTimeout: timer => timers.delete(timer),
    require: name => {
    if (name === 'vscode') return api;
    if (name === 'vscode-languageclient/node') return { LanguageClient, CloseAction: { DoNotRestart: 1 }, ErrorAction: { Shutdown: 2 } };
    return require(name);
  } });
  return { ...module.exports, context: { subscriptions }, commands, clients, watchers, events, errors, warnings, lines, state, timers,
    expireStartup: () => { for (const [timer, entry] of timers) { if (entry.milliseconds === 45000) { timers.delete(timer); entry.callback(); } } },
  };
}

module.exports = { scenario, deferred };
