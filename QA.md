# Local MVP Verification

Verified on macOS arm64 with Rust 1.97.1, Node 23.2.0, and an isolated VS Code
1.136.1 Extension Development Host. Minimum supported Rust/VS Code versions
have not been separately tested. Node LTS is recommended for development.

| Check | Command or surface | Result |
| --- | --- | --- |
| Rust unit and real stdio protocol tests | `cargo test --workspace --locked` | 48 tests pass: 21 formatter, 10 server unit, 17 protocol |
| Rust build | `cargo build -p iris-lsp --locked` | Pass |
| Rust lint | `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| Rust format | `cargo fmt --package iris-lsp --package iris-formatter -- --check` | Pass |
| Extension type check, bundle, editing, tokenizer and VM command tests | `npm test` in `extension/` | 68 tests pass |
| Real editor/server integration | `npm run test:integration` in `extension/` | Pass; host exits 0 |
| Existing VM execution | `iris --vm -e 'print("Iris tooling smoke")'` | Expected text, exit 0 |
| Upstream lexer/parser baseline | `cargo test --locked -p iris-lexer -p iris-parser` in Iris | 44 + 100 tests pass |

The editor test exercises actual extension activation and the real Rust binary,
an astral Unicode scalar before a bad continuation on a CRLF document, range
line 1 / UTF-16 column 18, clearing diagnostics after an unsaved replacement,
and completion of `typeof` and `yield`. Test profiles and workspaces are isolated
and removed afterwards. The downloaded editor cache is ignored by Git.

Protocol tests exercise startup, malformed requests, full synchronization,
diagnostic clearing, close/reopen, placeholder-position diagnostics, and exit
with stdin still open. Startup initially exceeded the five-second test limit;
a direct probe measured 15 seconds. Only the first response now allows 45
seconds; subsequent responses and shutdown remain bounded to five seconds.

The official editor test runner initially selected the old macOS executable
name `Electron`. The launcher now reads `CFBundleExecutable` from the downloaded
application's Info.plist. `VSCODE_EXECUTABLE_PATH` may select an existing editor.

## Remaining Verification Limits

- Rust/TypeScript LSP diagnostics are unavailable in the agent environment;
  installation was previously declined. Compiler, Clippy, and TypeScript checks
  were used instead. JSON is parsed by the tests; Markdown has no configured LSP.
- No screenshot-based visual review has been performed. TextMate tests use the
  actual Oniguruma tokenizer; the real editor integration validates behavior.
- The VM run command's argument boundaries, trust guard and save cancellation
  are unit-tested through a narrow VS Code API substitute. The real CLI was
  separately exercised; a VM task has not been asserted inside the editor host.
- Upstream GRAMMAR conformance could not build at baseline: existing lifetime
  errors in `iris-eval/src/lib.rs` and `iris-eval/src/source_runtime.rs`. No
  upstream source changes were made to address these unrelated errors.
- The editor emits unrelated keychain/agent-host warnings in this environment;
  the integration scenario and extension host exit successfully.

This is lexical tooling, not proof of program validity. See the server's
diagnostic location audit and extension limitations before extending coverage.

## Formatting And Editing Verification

The formatter and LSP now pass real editor checks for unsaved CRLF/astral-Unicode
documents, applying all edits, second-format no-op, and explicit format-on-save.
Saving an unclosed block preserves the input unchanged. VS Code may minimize a
server's full-document edit into several edits, so editor tests assert resulting
text rather than the number of protocol edits.

`IRIS_TEST_EMPTY=1 npm run test:integration` also passes: formatting and snippets
work without an open folder, while the previous VM run guard still refuses
before saving or launching a task. All eight snippets expand with real editor
tabstops and four-space defaults. The regex/action suite tests Enter rules;
live Enter automation remains unverified because typing commands produced no
document edits in this host. The optional `IRIS_EDITING_ENTER=1` case retains
that check rather than reporting it as a pass. No screenshot QA was performed.

With `IRIS_TEST_EXECUTABLE` pointing to the existing sibling `iris` binary, the
host test formats arithmetic/conditional and loop fixtures and executes original
and formatted files in isolated temporary directories. Both versions match
independent expected stdout, empty stderr, and zero exit status. Each CLI run is
bounded to ten seconds. Without that variable, runtime equivalence is explicitly
reported as skipped. The language source tree is not modified or rebuilt.

The formatter is indentation-only, with conservative whole-document skips and
bounded input/output/nesting. It preserves inline bytes and physical newlines;
it does not establish complete syntax validity or reformat literal interiors.
