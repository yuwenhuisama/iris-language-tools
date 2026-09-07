# iris-formatter

Conservative indentation-only formatting using the sibling `iris-lexer`.
No parser, evaluation, filesystem access, or command-line interface is involved.

```rust
use iris_formatter::{FormatOptions, FormatOutcome, format_document};

let options = FormatOptions::new(2, true).unwrap();
assert_eq!(
    format_document("fun f() {\nprint(1 + 2)\n}", options),
    FormatOutcome::Changed("fun f() {\n  print(1 + 2)\n}".into()),
);
```

`FormatOptions::new(tab_size, insert_spaces)` accepts sizes 1 through 16,
returning `InvalidTabSize` otherwise. Spaces use that many bytes per nesting
level; tabs use one tab per level. `format_document` returns `Changed(String)`,
`Unchanged`, or `Skipped(SkipReason)` without mutating the input.

Only leading ASCII spaces/tabs on token-led code lines may change. Braces,
`%{`, parentheses, and brackets contribute one level; consecutive leading
closers dedent their line. Generic angle brackets and inline whitespace are
untouched. Original LF, CRLF, lone CR, mixed endings, final-newline presence,
BOM, shebang, blank lines, and comment-only lines are preserved byte-for-byte.
Lines beginning inside a block comment, or with a comment before code, stay
untouched even when the lexer emits newline tokens inside the comment.

Until the lexer supplies reliable end spans, the whole source is skipped for
raw, triple, or interpolated literals, backslash-newline continuations, lexical
diagnostics, unmatched/unclosed delimiters, or uncertain protected ranges.
The literal/continuation preflight deliberately recognizes markers even inside
comments and ordinary literals: conservative false-positive skips are accepted.
Ordinary single-line strings, bytes, mutable strings, and regex literals can be
formatted without touching their contents. Numeric adjacency such as
`print(1+2)` currently produces an upstream lexical diagnostic and is skipped,
not repaired into `print(1 + 2)`.

Limits are 256 KiB input, 128 delimiter levels, and 1 MiB output. The generated
source is relexed, requiring identical token kinds and exactly mapped byte
offsets before returning a change. Comments are preserved by copying all bytes
outside eligible indentation prefixes, not by reconstructing trivia.

Run `cargo test -p iris-formatter` and
`cargo fmt --package iris-formatter -- --check` from the workspace root.
