const { runTests } = require('@vscode/test-electron');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');

async function main() {
  const extension = path.resolve(__dirname, '..');
  const cached = path.join(extension, '.vscode-test', `vscode-${process.platform}-${process.arch}-1.136.1`);
  const executable = process.env.VSCODE_EXECUTABLE_PATH ?? (process.platform === 'darwin'
    ? path.join(cached, 'Visual Studio Code.app/Contents/MacOS/Code')
    : path.join(cached, process.platform === 'win32' ? 'Code.exe' : 'code'));
  await fs.access(executable);
  const server = path.resolve(extension, '../target/debug', process.platform === 'win32' ? 'iris-lsp.exe' : 'iris-lsp');
  await fs.access(server);
  const root = await fs.realpath(await fs.mkdtemp(path.join(os.tmpdir(), 'ir-')));
  try {
    for (const mode of ['automatic', 'automatic-iris', 'missing', 'missing-iris', 'interrupt', 'timeout', 'deactivate']) {
      const recovery = mode.startsWith('missing');
      const retirement = ['interrupt', 'timeout', 'deactivate'].includes(mode);
      const run = path.join(root, mode);
      const workspace = path.join(run, 'workspace');
      const profile = path.join(run, 'p');
      await fs.mkdir(workspace, { recursive: true });
      await fs.mkdir(path.join(profile, 'User'), { recursive: true });
      const fixture = path.join(run, 'nonresponsive-server');
      if (retirement) {
        if (process.platform === 'win32') throw new Error('The nonresponding executable fixture requires a POSIX shebang host');
        await fs.writeFile(fixture, `#!${process.execPath}\n${await fs.readFile(path.join(__dirname, 'nonresponsive-server.cjs'), 'utf8')}`, { mode: 0o755 });
      }
      await fs.writeFile(path.join(profile, 'User/settings.json'), JSON.stringify({
        'iris.serverPath': retirement ? fixture : recovery ? path.join(run, 'missing-server') : server,
        'security.workspace.trust.enabled': false,
        'telemetry.telemetryLevel': 'off',
        'update.mode': 'none',
        'extensions.autoCheckUpdates': false,
        'extensions.autoUpdate': false,
      }));
      await runTests({
        vscodeExecutablePath: executable,
        extensionDevelopmentPath: extension,
        extensionTestsPath: path.join(__dirname, retirement ? 'startup-retirement.cjs' : 'startup-integration.cjs'),
        extensionTestsEnv: { IRIS_STARTUP_WORKSPACE: workspace, IRIS_STARTUP_SERVER: server, IRIS_STARTUP_RECOVERY: String(recovery), IRIS_STARTUP_MODE: mode, IRIS_STARTUP_FIXTURE: fixture, IRIS_STARTUP_SUFFIX: mode.endsWith('-iris') ? '.iris' : '.ir' },
        launchArgs: [workspace, '--user-data-dir', profile, '--extensions-dir', path.join(run, 'extensions'), '--disable-workspace-trust', '--skip-welcome', '--skip-release-notes', '--disable-updates'],
      });
    }
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
}

main().catch(error => { console.error(error); process.exitCode = 1; });
