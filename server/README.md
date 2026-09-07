# iris-lsp

Stdio LSP MVP with process-isolated formatting. Run `cargo build -p iris-lsp` from the workspace
root, then launch `target/debug/iris-lsp` without flags. Standard output is
reserved for Content-Length-framed JSON-RPC; startup failures use stderr.

## Scope

- Initialize/initialized, shutdown/exit, UTF-16 positions, full document sync.
- In-memory `file:` and `untitled:` documents. No filesystem reads or writes.
- Newer versions replace text; equal/stale versions and changes to closed
  documents are ignored. Ranged edits are rejected without mutating text.
- Keyword completion uses the shared `language/keywords.json` at build time.
- Lexer-only diagnostics on open/change; empty publication on fix or close.
- Document formatting through `iris-formatter`, using current unsaved text.
- Parser validation runs only in a formatting child, never in the main loop.
- No semantic analysis, evaluation, or user code execution.
- Full-document official-style formatting via `iris-formatter`, using the latest
  synchronized buffer and a UTF-16 full-document edit. Unchanged, unknown, closed
  or conservatively skipped documents return no edits; invalid options return
  `InvalidParams`. See `../formatter/README.md` for preservation rules and limits.

## Formatting

`documentFormattingProvider` is advertised at initialization. A
`textDocument/formatting` request accepts `tabSize` from 1 through 16 and a
boolean `insertSpaces`. Invalid options or parameters return JSON-RPC `-32602`.
These options are validated for protocol compatibility but do not override
official two-space indentation and the 120-column soft limit.
Changed documents receive one whole-document edit whose range uses the original
text's UTF-16 end position. Unknown/closed documents, unchanged text, and safety
skips return `[]`. Requests never modify the in-memory document; the client must
apply the edit and synchronize it normally. Equal/stale versions remain ignored.

The library checks original and formatted parser ASTs with source offsets
normalized. The server does not classify tokens or parse source itself. See
`../formatter/README.md` and `../STYLE.md` for style and safety boundaries.

Each job starts this executable's private `--format-worker` mode before LSP
stdio initialization. One active worker is allowed, with no pending queue;
additional requests return no edits and a structured `formatting.skipped` log.
Startup readiness has a 45-second deadline, separate from the two-second
computation/transport deadline. Source is limited to 256 KiB; JSON requests
allow bounded escaping overhead, and JSON output frames are limited to 1 MiB.
The worker's stdout carries only length-prefixed JSON, and stderr is discarded.
Parent-side failure logs carry request IDs and reasons, never source text.

Version and open-generation snapshots prevent edits after change, close, or
reopen. Cancellation returns `-32800`; stale, crashed, timed-out, and unsafe
jobs return `[]`. Cancellation, stale jobs, shutdown, exit, and disconnect all
kill/reap the child and join its transport thread. Completion remains available
while a formatting worker runs. Requests never mutate document text.

## Diagnostic Location Audit

The diagnostic allowlist is intentionally conservative. The companion lexer now
reports literal-opener offsets, but literal conversion codes have not all been
promoted to editor diagnostics; codes outside the allowlist remain logged rather
than receiving squiggles.

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
not exposed by `lex()` are not invented or reclassified. `LEX_RESOURCE_LIMIT`
also uses this log-only path. A shared 64-frame literal-boundary recursion budget
protects synchronous diagnostics as well as formatting; real open/change tests
verify survival and subsequent responsiveness. Future lexer changes
require re-auditing this allowlist; regression tests exercise all seven codes.

## Verification

```sh
cargo test --workspace
cargo fmt --package iris-lsp -- --check
cargo fmt --package iris-formatter -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
```

Protocol tests launch the real binary. The first response allows 45 seconds
for executable startup (local macOS runs observed 15-second launches). Formatting
responses allow 50 seconds for startup plus computation; other responses and
process exit are bounded to five seconds. Cleanup kills/reaps failed children
and joins response reader threads. Tests
leave stdin open during exit to catch shutdown hangs. `lsp-server` 0.10 stops
its reader on the exit notification, allowing the main process to drop the
connection and join transport threads without waiting for stdin EOF.

Unix-only injected-executable tests cover stalled workers, timeout boundaries,
crashes, cancellation, bounded admission, stale snapshots, shutdown, and reaping.
Real-binary protocol tests cover official style, UTF-16 edits, malformed input,
unchanged/closed documents, and document immutability on all platforms.

Use the package-scoped format command: `cargo fmt --all` recursively visits the
sibling path dependency's workspace and may report unrelated Iris formatting.
