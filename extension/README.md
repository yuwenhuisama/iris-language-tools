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

Open this `extension` directory in VS Code and press F5. In the development
window, set these user settings to your actual absolute paths:

```json
{
  "iris.serverPath": "/path/to/Project/iris-language-tools/target/debug/iris-lsp",
  "iris.executablePath": "/path/to/Project/Iris-Language/target/debug/iris"
}
```

Reload Window after changing the server path. Open a `.iris` file in a trusted
workspace. Both files and untitled Iris buffers receive lexical diagnostics
and keyword completion. The Output panel's `Iris Language Server` channel shows
startup errors and lexical errors whose source position is unavailable.

Use `Iris: Run File on VM` from the command palette for a saved local Iris file.
First use **File > Open Folder** to open the script's directory in the development
window. Running requires at least one workspace folder: VS Code restricts task
working directories in empty windows. Opening a file alone still supports LSP
features, but the run command will ask you to open a folder before saving or running.
Dirty files are saved first; a cancelled save cancels execution. The CLI runs
in the file's directory with `--vm`, using process arguments rather than a
shell command string. VM refusals remain visible; there is no evaluator fallback.

## Editing and Formatting

Iris defaults to four spaces (overridable). Conservative indentation and Enter
rules handle braces, hashes, parentheses and brackets. Lines with literal or
comment markers deliberately avoid extra indentation; these declarative rules
are not a complete parser for multiline lexical context.

Eight snippets are available through **Insert Snippet** or completion:
`let`, `fun`, `class`, `module`, `contract`, `if`, `while`, `for`.
Use Tab to move through the editable fields. A `fun` snippet is a method in its
class/module context; the contract snippet supplies a bodyless requirement.

Use **Format Document** (Shift+Alt+F on Windows) to normalize leading code
indentation. The server must be rebuilt after updating: `cargo build -p iris-lsp`
from the tools repository root, followed by **Developer: Reload Window**.
Formatting works on unsaved Iris buffers, including windows without a folder.

To format when saving, merge this into user settings (it is not enabled automatically):

```json
"[iris]": {
  "editor.defaultFormatter": "iris-local.iris-language-tools",
  "editor.formatOnSave": true
}
```

The formatter preserves existing line endings, line breaks and inline spacing.
It does not reflow comments, strings or long lines. Unsupported literal forms,
continuations, lexical errors and unbalanced delimiters produce no edits rather
than guessed changes. Selection formatting and on-type LSP formatting are not
implemented.

The whole document is conservatively left unchanged for raw, triple-quoted or
interpolated literals and explicit continuations. Marker checks can also skip a
document when those sequences occur inside a comment or ordinary string.
The limits are 256 KiB input, 128 delimiter levels and 1 MiB output; tab sizes
1 through 16 are supported. This is indentation formatting, not a pretty-printer.

Tests can also compare original/formatted programs on an existing CLI:

```sh
IRIS_TEST_EXECUTABLE=/absolute/path/to/iris npm run test:integration
IRIS_TEST_EMPTY=1 npm run test:integration
```

On PowerShell set `$env:IRIS_TEST_EXECUTABLE` or `$env:IRIS_TEST_EMPTY` before
running the same npm command. The tests never rebuild the original Iris project.

## Limitations

The server is lexical-only: no parser diagnostics, member completion, navigation,
rename or debugger. Keyword suggestions are not context-sensitive.
TextMate highlights keywords, comments, numbers, strings and operators; it does
not parse interpolation expressions or disambiguate contextual Regex versus
division. Raw/triple string highlighting is approximate, not a lexer validator.
Syntax colors follow the user's theme; the extension does not install a theme.

Restricted Mode disables executable activation and the run command. The executable
settings have machine scope so a repository cannot silently replace the chosen
server through workspace settings. No automatic server installation or update
is included. This is a local development extension, not a Marketplace release.
