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
Dirty files are saved first; a cancelled save cancels execution. The CLI runs
in the file's directory with `--vm`, using process arguments rather than a
shell command string. VM refusals remain visible; there is no evaluator fallback.

## Limitations

The server is lexical-only: no parser diagnostics, member completion, navigation,
rename, formatting or debugger. Keyword suggestions are not context-sensitive.
TextMate highlights keywords, comments, numbers, strings and operators; it does
not parse interpolation expressions or disambiguate contextual Regex versus
division. Raw/triple string highlighting is approximate, not a lexer validator.
Syntax colors follow the user's theme; the extension does not install a theme.

Restricted Mode disables executable activation and the run command. The executable
settings have machine scope so a repository cannot silently replace the chosen
server through workspace settings. No automatic server installation or update
is included. This is a local development extension, not a Marketplace release.
