const { runTests, downloadAndUnzipVSCode } = require('@vscode/test-electron');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs/promises');
const os = require('node:os');
const path = require('node:path');

async function main() {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'iris-editor-'));
  try {
    const workspace = path.join(root, 'workspace with spaces');
    await fs.mkdir(workspace);
    const server = path.resolve(__dirname, '../../target/debug', process.platform === 'win32' ? 'iris-lsp.exe' : 'iris-lsp');
    await fs.access(server);
    const user = path.join(root, 'profile');
    await fs.mkdir(path.join(user, 'User'), { recursive: true });
    await fs.writeFile(path.join(user, 'User/settings.json'), JSON.stringify({
      'iris.serverPath': server,
      'security.workspace.trust.enabled': false,
      'telemetry.telemetryLevel': 'off',
    }));
    let executable = process.env.VSCODE_EXECUTABLE_PATH;
    if (!executable) {
      executable = await downloadAndUnzipVSCode('1.136.1');
      if (process.platform === 'darwin') {
        const contents = path.resolve(path.dirname(executable), '..');
        const name = execFileSync('/usr/bin/plutil', ['-extract', 'CFBundleExecutable', 'raw', '-o', '-', path.join(contents, 'Info.plist')], { encoding: 'utf8' }).trim();
        executable = path.join(contents, 'MacOS', name);
      }
    }
    await runTests({
      vscodeExecutablePath: executable,
      extensionDevelopmentPath: path.resolve(__dirname, '..'),
      extensionTestsPath: path.join(__dirname, 'integration.cjs'),
      launchArgs: [workspace, '--user-data-dir', user, '--extensions-dir', path.join(root, 'extensions'), '--disable-workspace-trust', '--skip-welcome', '--skip-release-notes'],
    });
  } finally {
    await fs.rm(root, { recursive: true, force: true });
  }
}
main().catch(error => { console.error(error); process.exitCode = 1; });
