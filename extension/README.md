# Iris for VS Code (Local MVP)

Requires VS Code 1.82+, Node.js 20+ for development, and the sibling Rust server.
Node LTS is recommended. No executable is downloaded by the extension.

```sh
# From iris-language-tools/extension:
npm ci
npm test
npm run test:integration
```

Build the server first with `cargo build -p iris-lsp` from the repository root.
The integration command downloads an isolated VS Code via the official test
runner; it does not install the extension in your normal editor profile.

Open this `extension` directory in VS Code and press F5. The prelaunch tasks build
`iris-lsp` from the tools repository root with `cargo build --offline -p iris-lsp`,
then compile the extension. Cargo must be on VS Code's PATH and the Rust dependencies
must already be cached. The tasks do not change your selected server executable.
In the development window, set these user settings to your actual absolute paths:

```json
{
  "iris.serverPath": "/path/to/Project/iris-language-tools/target/debug/iris-lsp",
  "iris.executablePath": "/path/to/Project/Iris-Language/target/debug/iris"
}
```

Run **Iris: Restart Language Server** after changing the server path. Open a `.ir`
or `.iris` file in a trusted workspace to activate automatically. Both suffixes use
the same Iris v1 grammar; `.ir` does not enable archived legacy syntax.
Both files and untitled Iris buffers receive lexical diagnostics,
definitions, references, hover, symbol/member completion and type inlay hints. Standard
VS Code navigation, hover and completion work through the language client without
custom providers or click handlers. Hover a statically resolved symbol to see its
type or source method signature at the selected occurrence. Hover uses current
unsaved source, including dirty same-package targets; unknown members return no
hover rather than a guessed signature. Documentation comments are not attached to
analysis metadata yet, so hover does not include documentation text.
Type hints are enabled for Iris by default and can be changed with
`editor.inlayHints.enabled`. The Output panel's `Iris Language Server` channel shows
the selected startup command, initialized server name/version and availability of
formatting, definition, references, completion, inlay hints and hover, plus startup errors
and lexical errors whose source position is unavailable. Startup logging does not
include source text. If a server omits an expected capability, a warning suggests
checking for a stale or wrong executable; available features remain enabled.

Startup failures offer **Show Output**, **Open Settings** and **Retry**. Correct the
machine-scoped `iris.serverPath` in User settings, build the server if necessary,
then retry or run **Iris: Restart Language Server** without reloading the window.
Restarts are serialized, concurrent requests share one restart, and the previous
client and file watchers are disposed before replacement. Unexpected connection
closure is logged and requires an explicit restart. **Run File on VM** remains
independent of language-server startup. Empty output or a running extension alone
cannot establish the cause of missing providers; inspect the new startup evidence.
Initialization has a 45-second deadline. Restarting during initial activation or
deactivating interrupts a nonresponding server, closes its connection and retires
its process before replacement. Late initialization cannot restore its providers.

Use `Iris: Run File on VM` from the command palette for a saved local Iris file.
First use **File > Open Folder** to open the script's directory in the development
window. Running requires at least one workspace folder: VS Code restricts task
working directories in empty windows. Opening a file alone still supports LSP
features, but the run command will ask you to open a folder before saving or running.
Dirty files are saved first; a cancelled save cancels execution. The CLI runs
in the file's directory with `--vm`, using process arguments rather than a
shell command string. VM refusals remain visible; there is no evaluator fallback.

## Editing and Formatting

Iris defaults to two spaces (overridable while typing). Conservative indentation and Enter
rules handle braces, hashes, parentheses and brackets. Lines with literal or
comment markers deliberately avoid extra indentation; these declarative rules
are not a complete parser for multiline lexical context.

Eight snippets are available through **Insert Snippet** or completion:
`let`, `fun`, `class`, `module`, `contract`, `if`, `while`, `for`.
Use Tab to move through the editable fields. A `fun` snippet is a method in its
class/module context; the contract snippet supplies a bodyless requirement.

Use **Format Document** (Shift+Alt+F on Windows) to apply the official style in
`../STYLE.md`, always using two spaces and a 120-column soft limit. The server must be rebuilt after updating: `cargo build -p iris-lsp`
from the tools repository root, followed by **Iris: Restart Language Server**.
Formatting works on unsaved Iris buffers, including windows without a folder.

To format when saving, merge this into user settings (it is not enabled automatically):

```json
"[iris]": {
  "editor.defaultFormatter": "iris-local.iris-language-tools",
  "editor.formatOnSave": true
}
```

Named methods and control bodies expand; `else` starts a new line while `catch`
and `finally` stay attached when comments permit. Spacing, ordinary semicolons,
multiline lists and blank lines are normalized without reordering declarations.
Simple closures can remain inline; their header separator is retained.

Literal spellings and comment bodies are preserved. Exterior line endings become
LF with one final newline, but the editor refuses a result containing protected
CR bytes rather than normalize literal or block-comment contents globally.
Existing UTF-8 BOM encoding is controlled by VS Code's **Save with Encoding**,
not by LSP text edits; use UTF-8 without BOM for new source files.
Invalid or unsupported input, explicit continuations, parse disagreement and
resource limits produce no edits. Check **Iris Language Server** output for a
skip reason. Selection and on-type LSP formatting remain unimplemented.

The limits are 256 KiB input, 128 delimiter levels, and 1 MiB output, with a
two-second worker computation deadline after readiness. Valid editor indentation
options are accepted but cannot change the fixed official style. Update and
rebuild the companion Iris CLI too: the new layout requires the frontend's
newline/list and exact token-span repairs.

Tests can also compare original/formatted programs on an existing CLI:

```sh
IRIS_TEST_EXECUTABLE=/absolute/path/to/iris npm run test:integration
IRIS_TEST_EMPTY=1 npm run test:integration
```

On PowerShell set `$env:IRIS_TEST_EXECUTABLE` or `$env:IRIS_TEST_EMPTY` before
running the same npm command. The tests never rebuild the original Iris project.

`npm run test:startup` runs separate fresh native VS Code hosts for automatic `.ir`
and `.iris` recognition/provider availability, including real Hover results, and
missing-executable recovery for both suffixes after a User
setting correction. It never forces extension activation or reassigns the language.
It uses an existing `.vscode-test` cache for VS Code 1.136.1, or the executable set
in `VSCODE_EXECUTABLE_PATH`; it fails rather than downloading a missing editor.
These tests use isolated temporary profiles, not your normal User settings.
On POSIX hosts the startup suite also launches a local nonresponding executable
to check interruption, the full 45-second deadline, deactivation and corrected
restart. Controlled transports exercise late initialize replies and pending
initialized writes against the installed language client. The executable fixture
requires a POSIX shebang host and does not download a runtime.

The integration suite calls `vscode.executeHoverProvider` against the real server
for untitled Integer types, current-buffer type changes, annotated source method
signatures, exact UTF-16 occurrence ranges and unknown-member refusal. Same-package
tests change an unsaved target return type from Integer to String and require the
caller's hover to update while retaining the caller's range, without saving the target.
To run against an existing editor without downloads, set `VSCODE_EXECUTABLE_PATH`
to the cached native VS Code executable before `npm run test:integration`.

## Limitations

Semantic queries use current unsaved buffers. Cross-file queries use sources
selected by `iris.toml` within the same manifest group; source and manifest file
changes (`.ir`, `.iris` and `iris.toml`) refresh the index. Standalone buffers remain isolated. Known receiver
types provide member completion, including incomplete trailing-dot edits.
Omitted named-method parameter and return annotations display `Dynamic<Object>`;
local types are inferred conservatively, and explicit annotations get no redundant
hints. External packages and uncertain composed member surfaces remain unsupported.
There are no parser diagnostics, rename or debugger features. Keywords are
suppressed in protected text and member/namespace completion contexts.
TextMate highlights keywords, comments, numbers, strings and operators; it does
not parse interpolation expressions or disambiguate contextual Regex versus
division. Raw/triple string highlighting is approximate, not a lexer validator.
Syntax colors follow the user's theme; the extension does not install a theme.

Restricted Mode disables executable activation and the run command. The executable
settings have machine scope so a repository cannot silently replace the chosen
server through workspace settings. No automatic server installation or update
is included. This is a local development extension, not a Marketplace release.
