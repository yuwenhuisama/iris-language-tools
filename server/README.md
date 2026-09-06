# iris-lsp

Synchronous stdio LSP MVP. Run `cargo build -p iris-lsp` from the workspace
root, then launch `target/debug/iris-lsp` without flags. Standard output is
reserved for Content-Length-framed JSON-RPC; startup failures use stderr.

## Scope

- Initialize/initialized, shutdown/exit, UTF-16 positions, full document sync.
- In-memory `file:` and `untitled:` documents. No filesystem reads or writes.
- Newer versions replace text; equal/stale versions and changes to closed
  documents are ignored. Ranged edits are rejected without mutating text.
- Keyword completion uses the shared `language/keywords.json` at build time.
- Lexer-only diagnostics on open/change; empty publication on fix or close.
- No parser, semantic analysis, evaluation, or code execution.

## Diagnostic Location Audit

The sibling `Iris-Language/crates/iris-lexer/src/scanner.rs` was audited before
implementation. Its literal conversion at lines 102-115 returns placeholder
offsets (zero, or the BOM width), not the location of the invalid literal.
Those codes must not produce editor squiggles.

The precise-location allowlist contains only scanner-origin codes:

| Code | Scanner origin |
| --- | --- |
| `LEX_SHEBANG_NOT_FIRST` | Current token offset |
| `LEX_UNTERMINATED_COMMENT` | Comment opener |
| `LEX_BAD_CONTINUATION` | Backslash |
| `LEX_INTERPOLATION_OUTSIDE_LITERAL` | Interpolation opener |
| `LEX_UNTERMINATED_LITERAL` | Scanner literal opener |
| `LEX_INVALID_IDENTIFIER` | Rejected scalar |
| `LEX_BAD_LITERAL_PREFIX` | Literal prefix |

Diagnostics use zero-width ranges because upstream provides no length. Byte
offsets are mapped against original document text, counting UTF-16 code units,
including BOM and astral scalars, and recognizing LF, CRLF, and lone CR.
Upstream line/column fields are not used. Off-boundary offsets are not guessed.

All other codes become `window/logMessage` warnings containing a JSON object
with event `lexer.unlocated`, code, URI, and document version. No source text is
logged. This includes literal numeric/escape/fence/indent errors. Lexer warnings
not exposed by `lex()` are not invented or reclassified. Future lexer changes
require re-auditing this allowlist; regression tests exercise all seven codes.

## Verification

```sh
cargo test --workspace
cargo fmt --package iris-lsp -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
```

Protocol tests launch the real binary. The first response allows 45 seconds
for executable startup (local macOS runs observed 15-second launches); later responses and
process exit are bounded to five seconds. Cleanup kills/reaps failed children. Tests
leave stdin open during exit to catch shutdown hangs. `lsp-server` 0.10 stops
its reader on the exit notification, allowing the main process to drop the
connection and join transport threads without waiting for stdin EOF.

Use the package-scoped format command: `cargo fmt --all` recursively visits the
sibling path dependency's workspace and may report unrelated Iris formatting.
