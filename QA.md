# Local MVP Verification

## Hover Milestone

Hover implementation passed its scoped final gate. It reuses resolved occurrences,
shows body-free source signatures and conservative type labels, and returns the
request document's exact UTF-16 occurrence range. Unknown, ambiguous and protected
positions return no Hover. Documentation comments are not displayed because the
source graph does not carry reliable attachment metadata.

- Full locked/offline serial Rust workspace tests passed after Hover integration:
  328 tests (77 analysis, 105 formatter, 146 server).
- Workspace all-target build, strict Clippy and package formatting check passed.
- The public `iris-analysis` Hover example passed its signature and boundary checks.
- The initial real-editor test reached Hover successfully but incorrectly matched
  unescaped signature text against raw Markdown. The adapter now decodes Markdown
  punctuation escapes for display-equivalent assertions without changing server
  escaping or range checks. The parent reran real editor integration successfully:
  local Integer/String changes, method signatures, unknown-member refusal and
  dirty cross-file updates all passed with exact caller occurrence ranges.
- Parent reruns of normal and empty editor integration passed. All seven cached
  startup hosts also passed with Hover enabled, including both suffixes, missing
  executable recovery, interruption, timeout and deactivation/late-transport checks.
- Oracle returned APPROVE with high confidence for the documented static scope.
  It independently passed 23 analysis Hover tests, nine renderer/coordinator tests,
  15 real-stdio Hover tests and the public API example. Physical mouse gestures,
  screenshot appearance and Windows execution were not separately verified;
  real VS Code provider commands exercised the feature on macOS.

## Suffix And Startup Repair

The user's Windows trace showed a successful initialize handshake advertising
only keyword completion, with no formatting or semantic navigation capabilities.
Their subsequent build failed on missing `iris_parser::source`, `parse_editor`,
and `Token.end`, identifying an outdated sibling frontend checkout. This explains
the absent providers; it does not establish support for dynamic/external members
in their HTTP demo. The user subsequently reported recovery; this is user-supplied
Windows evidence, not a locally observed Windows run.

Current local verification on macOS with cached VS Code 1.136.1:

- `cargo test --workspace --locked --offline -- --test-threads=1`: 281 pass
  (54 analysis, 105 formatter, 122 server).
- `npm test`: 83 pass, including TypeScript checking and bundling.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: pass.
- `npm run test:startup`: five fresh editor hosts passed, using no downloads and
  no forced language mode or explicit extension activation. Opening an actual
  `.ir` file registers all five providers; exact references and member completion
  work, and nonempty formatting edits apply. `.iris` remains supported.
- The second host starts with a nonexistent executable. Correcting User settings
  and invoking `iris.restartLanguageServer` restores providers without reloading.
- Mixed-suffix real-stdio tests cover both manifest directions, disk changes,
  unsaved overlays, and bounded inventory error details without source contents.
- Normal and empty-workspace `test:integration` runs passed after the startup
  changes, preserving semantic queries, formatting, snippets and the run guard.
  Optional VM-equivalence scenarios were explicitly skipped because no CLI path
  was supplied; no new runtime-execution result is claimed.

Normal startup and missing capabilities are now logged. Restart serialization,
cleanup failures, deactivation races and trust refusal have boundary tests.
Restricted Mode was not exercised in a real host; trust checks remain unchanged.
F5 now builds the known development server before compiling the extension, but
does not override the configured command. The first gate rejected an unbounded
initialize wait: a launched server that never replies blocks restart and delays
deactivation cleanup indefinitely. The fix adds a 45-second initialization limit
and interruptible retirement with owned child-process cleanup. Real nonresponding
children were retired on timeout, restart and deactivation; corrected restarts
restored definitions. Two installed-languageclient transport probes verified
that late initialize responses and initialized writes register no stale providers.
These new executable fixtures require POSIX and have not been run on Windows.
The focused Oracle re-review returned APPROVE with high confidence against
vscode-languageclient 9.0.1. It independently passed all 83 extension tests,
all five cached VS Code startup hosts, and both late-response/write probes.
No unresolved blocker remains from the startup review; the approval does not
extend the platform coverage beyond the local macOS/POSIX evidence above.

## Semantic Editor Milestone

The following evidence covers the uncommitted semantic-editor implementation.
The formatter-era evidence below is retained as historical context; its approval
does not approve this new milestone. The first semantic gate returned REJECT:
nominal header annotations and rest parameter annotations fabricated instance
receiver types, while valid discard parameters marked callable scopes damaged.
Those issues now have parser metadata fixes, category-aware binding types, and
real-protocol regressions across all four providers. Independent reruns passed
273 tools tests and 164 parser tests; the normal editor integration was refreshed
successfully, including a subsequent empty-workspace run. The focused Oracle
re-review returned APPROVE with high confidence for the three fixes. It
independently passed 54 analysis tests, five real-stdio tests covering 14 receiver
scenarios plus discards, and six focused parser tests. One parser launch timed
out before test output; the unchanged retry passed. No unresolved blocker remains
from the semantic gate findings. Approval is limited to the documented static
editor scope and does not certify unsupported resolution or untested platforms.

| Check | Command or surface | Result |
| --- | --- | --- |
| Tools workspace | `cargo test --workspace --locked -- --test-threads=1` | 273 pass: 54 analysis, 105 formatter, 114 server |
| Strict Rust lint | `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| Rust formatting | `cargo fmt -p iris-analysis -p iris-lsp -p iris-formatter -- --check` | Pass |
| Extension | `npm test` | 68 pass, including TypeScript checking and bundle |
| Real VS Code normal workspace | `npm run test:integration` | All four providers and cross-file scenarios pass |
| Real VS Code empty workspace | `IRIS_TEST_EMPTY=1 npm run test:integration` | All four single-file providers and existing run guard pass |
| Runtime regression | `cargo test -p iris-vm -p iris-eval --lib --locked` in sibling checkout, isolated target | 742 pass, 1 ignored |

The real editor scenarios are in `extension/test/semantic-integration.cjs` and
`semantic-workspace.cjs`. They verify exact UTF-16 definitions, references excluding
shadowed locals, prefix replacement, known instance-member completion, incomplete
trailing dots, local Integer hints, omitted method contracts as Dynamic<Object>,
and no redundant hints on written annotations. A manifest-backed two-file fixture
verifies disk source/manifest watches and unsaved target edits updating definition
positions, references, candidates and return-type hints without saving or execution.
The parent also exercised all four providers through a direct real stdio client.

The latest editor runs deliberately left `IRIS_TEST_EXECUTABLE` unset: optional
VM-formatting equivalence was reported as skipped, not passed. Semantic fixtures
are never executed. Definition-provider behavior is tested through real VS Code
commands; physical modifier-click gestures and screenshot rendering of hint text
have not been separately verified. Windows and minimum-version limits still apply.

Cross-file coverage is same-package manifest-listed source only; standalone files
remain isolated. External package imports, inheritance/mixin composition, generic
substitution and runtime-added members are not resolved speculatively. References
are statically resolved occurrences, not all potential dynamic calls. Unknown
types produce no concrete hint. Syntax-error recovery is intentionally limited.

## Earlier Formatter Verification

Verified on macOS arm64 with Rust 1.97.1, Node 23.2.0, and an isolated VS Code
1.136.1 Extension Development Host. Minimum supported Rust/VS Code versions
and Windows platforms have not been separately tested. Node LTS is recommended
for development.

| Check | Command or surface | Result |
| --- | --- | --- |
| Rust unit and real stdio protocol tests | `cargo test --workspace --locked -- --test-threads=1` in `iris-language-tools` | 152 tests pass: 105 formatter, 47 server (27 unit, 10 formatting integration, 2 recursion integration, 8 protocol) |
| Rust build | `cargo build -p iris-lsp --locked` | Pass |
| Rust lint | `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| Rust format | `cargo fmt --package iris-lsp --package iris-formatter -- --check` | Pass |
| Extension type check, bundle, editing, tokenizer and VM command tests | `npm test` in `extension/` | 68 tests pass |
| Real editor/server integration | `npm run test:integration` in `extension/` | Pass; host exits 0 |
| Empty-workspace editor integration | `IRIS_TEST_EMPTY=1 npm run test:integration` in `extension/` | Pass; host exits 0 |
| VM execution equivalence | `IRIS_TEST_EXECUTABLE=... npm run test:integration` | Pass; arithmetic and loop programs match expected stdout |
| Upstream lexer/parser | `cargo test --locked -p iris-lexer -p iris-parser` in `Iris-Language` | 174 tests pass: 62 lexer (44 + 12 + 6), 112 parser (100 + 10 + 2) |
| Companion CLI rebuild | `cargo build -p iris-cli --locked` with isolated validation target directory | Pass after concurrent VM work resumed compiling |
| Upstream VM and evaluation libraries | `cargo test -p iris-vm -p iris-eval --locked --lib` in `Iris-Language` | 742 tests pass (710 eval + 32 VM), 1 ignored |
| Upstream VM/evaluation package integration | `cargo test -p iris-vm -p iris-eval --locked -- --test-threads=1` with isolated validation target directory | 769 pass, 1 ignored; includes real native module calls |

The editor test exercises actual extension activation and the real Rust binary,
an astral Unicode scalar before a bad continuation on a CRLF document, range
line 1 / UTF-16 column 18, clearing diagnostics after an unsaved replacement,
and completion of `typeof` and `yield`. Test profiles and workspaces are isolated
and removed afterwards. The downloaded editor cache is ignored by Git.

Protocol tests exercise startup, malformed requests, full synchronization,
diagnostic clearing, close/reopen, placeholder-position diagnostics, and exit
with stdin still open. Startup initially exceeded the five-second test limit;
a direct probe measured 15 seconds. Only the first response now allows 45
seconds; formatting responses allow 50 seconds for worker startup and computation.
Other responses and shutdown remain bounded to five seconds.

The official editor test runner initially selected the old macOS executable
name `Electron`. The launcher now reads `CFBundleExecutable` from the downloaded
application's Info.plist. `VSCODE_EXECUTABLE_PATH` may select an existing editor.

## Remaining Verification Limits

- Rust and TypeScript LSP diagnostics are unavailable in the agent environment;
  installation was previously declined and no servers were installed. Compiler,
  Clippy, and TypeScript checks were used instead. JSON is parsed by the test
  suite; Markdown has no configured LSP server.
- No screenshot-based visual review has been performed. TextMate tests use the
  actual Oniguruma tokenizer; real editor integration validates behavior.
- Live Enter automation remains unverified because typing commands produced no
  document edits in this host. The optional `IRIS_EDITING_ENTER=1` check retains
  that probe rather than reporting an unearned pass.
- Windows platforms and minimum supported toolchain versions are unverified.
- An earlier VM/evaluation integration run timed out at 120 seconds while
  building its native fixture. The final serial package-level run passed all
  769 tests with one ignored, including the native fixture. This is not a full
  upstream workspace or specification-conformance run.
- An earlier CLI rebuild failed during concurrent, unrelated VM edits. A later
  rebuild succeeded, the 742 library tests were rerun successfully, and the
  latest editor runtime-equivalence run passed with that rebuilt CLI. This task
  did not modify the concurrent VM work and does not claim full conformance.
- Process startup is intermittently slow in this environment. One parallel
  tools run timed out during all ten formatting-test server initializations;
  a complete serial rerun passed without changing assertions or deadlines.
  A later serial rerun again timed out in three server initializations (the
  other seven formatting integration cases passed). Thus 121 is a completed
  passing run, not a claim that process-based tests are consistently green.
  A final focused rerun passed all ten formatting integration tests and both
  deep-interpolation LSP survival tests without code or deadline changes.
  A Node-spawned public example also timed out before output, while the same
  input via `cargo run` returned the expected result.
  One rebuilt-CLI editor run reached its ten-second process timeout; a direct
  CLI smoke run and subsequent full editor/runtime retry passed unchanged.
- The companion frontend source was modified in `Iris-Language`, so both binaries
  (`iris-lsp` and the `iris` CLI) require rebuilding together.
- The first formal gate review rejected the change: deeply nested interpolation
  could overflow the lexer stack in the main LSP process; split declaration
  headers, mutation-selector spacing, comparison continuations, and comment
  attachments also had reproduced layout defects. All four findings now have
  fixes and passing regressions; the counts above include them. The second
  review confirmed those exact fixes but rejected two neighboring cases:
  trailing comments after operators erase continuation indentation, and list
  elements with both leading and later comments can place commas on separate
  lines. Both now have focused fixes and eight additional passing regression
  tests. The 129-test run, build, Clippy, format check and real editor/runtime
  integration passed. The third review confirmed those fixes but found that a
  trailing comment after a generic type prevents generic recognition and leaks
  continuation indentation into the following declaration. A focused lookahead
  fix and 13 more exact-output/idempotence tests now pass, including genuine
  comparisons and comment-contained newlines. The final 142-test run, build,
  Clippy and format check passed. The fourth review confirmed the original
  generic case but found stale spacing state after a multiline block comment's
  embedded statement boundary, producing `- next()` instead of unary `-next()`.
  That boundary fix now passes ten additional regressions, including delimited
  binary expressions and pending operands. The implementation worker stalled
  during compaction and was cancelled; the parent completed lint cleanup and
  independently verified all 152 tests, build, Clippy and format checks. The
  public reproducer now retains unary `-next()`. The final focused Oracle gate
  returned APPROVE with high confidence: all 105 formatter tests passed
  independently, nine additional public-driver outputs were idempotent, and
  invalid input was safely refused. No unresolved blocker remains from the
  review findings. This approval covers the reviewed implementation and fixes,
  not untested platforms or full language conformance.

The lexer now shares a 64-frame budget across string, regex and interpolation
boundary scanning. Six lexer tests include bounded subprocess cases and normal
nesting/sibling coverage. Two real LSP tests send the 50,009-byte deep-interpolation
payload through open/change, observe `LEX_RESOURCE_LIMIT` logging, and verify
completion, subsequent requests, repaired text and shutdown. Twenty-four formatter
goldens cover the corrected headers, suffixes, continuations and comment placement,
including second-format no-op and neighboring syntax.

This is lexical tooling and an official style formatter, not a guarantee of full
program validity or semantic conformance. See the server diagnostic audit and
extension limitations before extending coverage.

## Formatting And Editing Verification

Official formatting follows STYLE.md: two spaces, a 120-column soft limit,
multiline named methods and control bodies, newline-before-`else`, same-line
`catch` and `finally`, and ordinary statement-semicolon removal. Closure header
separators and property accessor semicolons are retained. It is not an indentation-only or
four-space formatter. Formatting runs in a bounded worker with AST and token
preservation checks before any edit is returned.

The formatter and LSP pass real editor checks across both normal and empty
VS Code 1.136.1 test runs:
- Unsaved LF documents with astral Unicode format cleanly to official style.
- Re-running the formatter is idempotent and emits zero edits.
- Protected literal CRLF in multiline strings produces no edits, preventing the
  editor model from corrupting raw literal newlines.
- A CRLF file saved on disk formats and saves cleanly as LF.
- Incomplete sources with unclosed delimiters safely no-op on save without loss.
- VS Code may minimize the full replacement into smaller edits; tests assert
  final text rather than edit count.

`IRIS_TEST_EMPTY=1 npm run test:integration` also passes. Formatting and snippets
work without an open folder, while the VM run guard safely refuses before saving
or launching a task. All eight snippets expand with real editor tabstops and
configured two-space defaults.

With `IRIS_TEST_EXECUTABLE` pointing to the rebuilt companion CLI (`/var/folders/tq/hgbd_kk13_1bgdj9c8983s8h0000gn/T/opencode/iris-style-validation/debug/iris`),
the host test formats arithmetic/conditional and loop fixtures and executes both
original and formatted versions in temporary directories. Both versions match
expected stdout (`3\n` and `0\n1\n2\n`), with empty stderr and zero exit status.
Each run is bounded to ten seconds. Without this variable, runtime equivalence
is reported as skipped. Intermediate normal and empty host runs passed with
runtime equivalence explicitly skipped; the final runs in both modes passed
with the rebuilt CLI and runtime equivalence enabled. One
empty-host rerun timed out at 120 seconds before reporting any test results;
a retry with a 300-second harness timeout completed successfully.
