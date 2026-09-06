# Iris Language Tools

Development repository for Iris v1 editor tooling. The Rust language
server and VS Code extension are separate from the language implementation.
This is an initial MVP, not a published extension or a complete semantic IDE.

## Development Layout

Keep [Iris-Language](https://github.com/yuwenhuisama/Iris-Language) and these
tools as sibling checkouts:

```text
Project/
  Iris-Language/
    crates/iris-lexer/
  iris-language-tools/
    language/keywords.json
    server/
    extension/
```

The server uses a local Cargo path dependency on the existing `iris-lexer`.
No compiler source is copied into this repository. The inspected language
checkout was at `91c1b8dd68e69978c6b268a5d487017396c70737` with existing local
changes; this is provenance, not a pinned dependency. The sibling checkout's
current sources are used when building.

The keyword inventory follows `IRIS-V1-GRAMMAR-C013` and the `C072` erratum:
50 reserved words, including `typeof` and `yield`. Legacy `.ir` syntax and
historical Notepad++ highlighting are not the v1 grammar authority.

## MVP Boundary

- Stdio LSP with full synchronization of open, unsaved documents.
- UTF-16 positions and located lexical diagnostics from the existing lexer.
- Keyword completion, not type-directed or member completion.
- VS Code `.iris` registration, TextMate highlighting, and VM run command.

The upstream parser currently reports codes without source ranges, so parser
and static-analysis diagnostics are not exposed yet. Some upstream literal
diagnostics use a placeholder offset; those must be logged as unlocated
rather than presented as misleading start-of-file squiggles. A clean Problems
panel is therefore not a guarantee that the program parses or runs.

No user program is executed for diagnostics or completion. The explicit run
command is separate and uses the `iris` CLI's `--vm` mode. Executable startup
requires a trusted VS Code workspace.

Definitions, references, rename, semantic tokens, formatting, workspace package
resolution, and debugging are not implemented in this MVP. They require
source ranges, recovery, and a queryable semantic layer in the language front
end rather than inspection of runtime state.

## Build and Test

From this repository, with Rust 1.93 or newer and a compatible Node.js installed:

```sh
cargo build -p iris-lsp
cargo test --workspace
cargo fmt --package iris-lsp -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

See `extension/README.md` for extension setup, test commands, and launching an
Extension Development Host. Configure `iris.serverPath` to the absolute path
of `target/debug/iris-lsp` (or `iris-lsp.exe` on Windows). Configure
`iris.executablePath` to the Iris CLI executable to enable explicit VM runs.

Source is hosted at https://github.com/yuwenhuisama/iris-language-tools.
No Marketplace extension or packaged binary release is available yet.
