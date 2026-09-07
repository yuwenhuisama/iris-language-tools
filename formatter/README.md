# iris-formatter

Reusable, non-evaluating Iris v1 formatter using the sibling lexer, parser, and
syntax crates. The style is fixed: two spaces and a 120-character soft width.
Editor indentation options do not change it.

```rust
use iris_formatter::{FormatOutcome, format_document};

assert_eq!(
    format_document("fun f(){print(1+2);}"),
    FormatOutcome::Changed("fun f() {\n  print(1 + 2)\n}\n".into()),
);
```

## API

`format_document(source: &str) -> FormatOutcome` returns:

- `Changed(String)`: the complete formatted document, after safety verification.
- `Unchanged`: the source already has the selected layout.
- `Skipped(SkipReason)`: no replacement is available; retain the original input.

There is no `FormatOptions` argument and no style configuration. The library
does not access the filesystem, execute Iris, spawn processes, or log.

`SkipReason` distinguishes `InputLimit`, `OutputLimit`, `NestingLimit`,
`Continuation`, `LexicalDiagnostics`, `MismatchedDelimiter`, `UnclosedDelimiter`,
`UncertainProtectedRange`, `ParseDiagnostics`, `CandidateParseDiagnostics`,
`SemanticMismatch`, and `TokenMismatch`.

## Formatting

Named methods, declarations and control bodies are multiline, with their opening
brace on the header line. `else` begins a new line; `catch` and `finally` join the
previous closer unless an intervening comment prevents that. Simple expression
closures can remain inline. Ordinary semicolons become statement boundaries;
closure header separators (including multiline closures) and property accessor
semicolons remain where required.

Broken argument, parameter, array, tuple and hash lists use one item per line,
two-space continuation indentation, and trailing commas where allowed. Lists
also break when their compact representation exceeds the soft width. Singleton
tuple commas remain significant; unnecessary inline list trailing commas are
removed. Indivisible literals and comments may exceed the soft width.

Operators and arrows are spaced, unary and parameter-channel prefixes remain
tight, colons and commas have a following space, and member paths and generic
brackets remain tight. Named declarations and methods have one separating blank
line; properties and imports remain grouped. Doc comments and decorators attach
to declarations. Nothing is sorted or reordered.

Ordinary line-comment markers have one following space and trailing comments
have two preceding spaces. Comment bodies are not reflowed. All literal bytes,
including raw fences, triple quotes, interpolation, regexes and interior CRLF,
are copied from exclusive lexer spans. Block-comment bytes are preserved.
Exterior newlines become LF, a leading BOM is removed, and nonempty output has
exactly one final newline. Whitespace-only input becomes empty.

## Safety

Input is limited to 256 KiB, delimiters to 128 levels and output to 1 MiB.
Explicit backslash continuations outside literals and comments are currently
no-edit cases. Malformed programs are rejected before layout, never repaired.
Both the original and candidate must be completely accepted by the parser.
Full syntax trees are compared after an exhaustive traversal normalizes only
`Raise.offset`. Declaration order, operators, annotations, values, defaults,
decorator arguments, catches and causes remain part of that comparison.
Literal/comment sequences and non-layout tokens are checked independently.
Unsupported or uncertain transformations return a typed skip rather than an edit.

Upstream parser resource limits may reject input below the formatter limits.
Hosts requiring a hard wall-clock deadline should run formatting in an isolated
worker process; the library does not supply a timeout or cancellation boundary.

## Verification

```sh
cargo test -p iris-formatter
cargo clippy -p iris-formatter --all-targets -- -D warnings
cargo fmt -p iris-formatter -- --check
cargo run -p iris-formatter --example format -- 'fun f(){print(1+2);}'
```

The example is a small library driver: one argument is source text; a skip is
reported on stderr with a nonzero status. Tests include golden layouts,
idempotence, syntax rejection, protected bytes, semantic boundaries and resource
budgets.
