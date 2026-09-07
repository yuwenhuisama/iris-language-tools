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
    formatter/
    server/
    extension/
```

The formatter uses local Cargo path dependencies on `iris-lexer`, `iris-parser`
and `iris-syntax` from the language repository.
No compiler source is copied into this repository. The inspected language
checkout was at `91c1b8dd68e69978c6b268a5d487017396c70737` with existing local
changes; this is provenance, not a pinned dependency. The sibling checkout's
current sources are used when building.

The keyword inventory follows `IRIS-V1-GRAMMAR-C013` and the `C072` erratum:
50 reserved words, including `typeof` and `yield`. Legacy `.ir` syntax and
historical Notepad++ highlighting are not the v1 grammar authority.

## Features and Scope

- Stdio LSP with full synchronization of open, unsaved documents.
- UTF-16 positions and located lexical diagnostics from the scanner allowlist.
- Static semantic navigation: Go to Definition (`F12` / `Ctrl+Click`) and Find References (`Shift+F12`).
- Smart completion for variables, parameters, constants, source classes, modules, contracts, type aliases, and known typed members alongside keywords.
- Type inlay hints showing local variable types, literal types, and explicit method return annotations.
- Same-package cross-file resolution using manifest source lists from `iris.toml`.
- Official-style document formatting through a bounded, isolated LSP worker.
- VS Code `.iris` registration, TextMate syntax highlighting, Enter/indentation rules, and eight snippets.
- Explicit Iris CLI `--vm` run command for trusted workspaces.

### Static Semantic Support

Static queries run in a dedicated background worker thread over immutable analysis snapshots:

- **Definition (`F12` / `Ctrl+Click`)**: Jumps to the exact declaration site of symbols across open buffers and same-package files. Unknown targets return empty results without guessing.
- **References (`Shift+F12`)**: Finds statically resolved symbol references across the package resolution group. References are static symbol bindings, not all possible dynamic runtime call targets.
- **Completion**: Offers in-scope identifiers, known member access, and keywords. Keywords are suppressed inside member dots, namespace qualifiers, string literals, and comments. Incomplete recovery regions retain replacement spans.
- **Inlay Hints**: Shows local inferred types for unannotated bindings and literals. Omitted method parameter and return annotations remain `Dynamic<Object>` under `TYPES-C003`; method bodies do not create inferred signature hints.
- **Workspace Packages**: Discovers `iris.toml` manifests and tracks explicit `sources` lists. Open unsaved editor buffers serve as overlays that take precedence over disk. Standalone files and conflicting package declarations remain safely isolated. The server reads manifest sources when needed and never writes to disk.

### Boundaries and Limitations

- **No runtime execution for analysis**: Diagnostics and queries inspect static source graph facts only. Code is never evaluated or executed to resolve types or symbols.
- **Parser recovery scope**: Recovery in `iris-parser` preserves usable declarations across trailing EOF block braces and incomplete dot receivers (`receiver.`). Partial namespace identifiers (`Namespace::Prefix`) resolve, but bare trailing namespace separators (`Namespace::`) without an identifier production are suppressed.
- **Conservative resolution**: Inherited members, mixin composition, contract views, generic type substitution, re-export facades, dynamic monkey patching, and external package imports are not synthesized or guessed.
- **Diagnostics**: Real-time editor diagnostics remain lexer-only scanner errors. Parser syntax errors do not emit squiggles in the editor.
- **File watching**: Protocol support for watched file changes is present on the server, while extension client file watchers are still in progress. Saving buffers or modifying workspace folders reloads the workspace snapshot.
- **Tooling in progress**: Rename, semantic token highlighting, and interactive debugging are not yet implemented.

```toml
# Example iris.toml manifest for same-package resolution
manifest_version = 1
package_id = "org.example.app"
api_major = 1
version = "0.1.0"
iris_major = 1
sources = [
  "main.iris",
  "utils.iris"
]
entry_modules = ["Main"]

[permissions]
required = []
optional = []
```

## Formatting

The canonical rules are in [STYLE.md](STYLE.md): two spaces, a 120-column soft
limit, multiline named methods, newline-before-`else`, same-line `catch` and
`finally`, and no ordinary statement-ending semicolons. The formatter also
normalizes spacing, wraps lists with legal trailing commas, separates named
declarations, and preserves literal contents and source order.

Use **Format Document** in VS Code. Formatting operates on the current unsaved
buffer. Editor tab/space preferences do not override the official layout.
The original and candidate must parse to the same AST, ignoring source locations,
and pass additional literal/comment/token preservation checks before edits are returned.

To opt into format-on-save, merge this into user settings:

```json
"[iris]": {
  "editor.defaultFormatter": "iris-local.iris-language-tools",
  "editor.formatOnSave": true
}
```

Raw, triple-quoted and interpolated literals are preserved, not rewritten.
Explicit line continuations and unsupported or uncertain syntax remain no-edit
cases. The formatter does not repair invalid code. VS Code may also refuse an
edit when its document-wide EOL model would change protected CRLF contents.
Existing encoding-level BOM removal is not guaranteed by an LSP text edit.

This version requires the companion frontend changes in `Iris-Language`: exclusive
token end spans, corrected numeric/literal boundaries, and contextual newline
parsing. Rebuild both `iris-lsp` and the `iris` CLI after updating both checkouts;
older CLIs can reject the new layout, notably newline-before-`else` and multiline lists.

## Upgrade and Rebuild

Building the language server depends directly on the sibling `Iris-Language` parser,
syntax, and lexer crates. When updating either checkout, rebuild the server to ensure
binary compatibility with the companion frontend:

```sh
cargo build -p iris-lsp
```

Rebuild the companion `iris` CLI in `Iris-Language` as well if you use VM run commands
or formatted output that relies on updated grammar rules.

## Build and Test

From this repository, with Rust 1.93 or newer and a compatible Node.js installed:

```sh
cargo build -p iris-lsp
cargo test --workspace
cargo fmt --package iris-lsp --package iris-formatter -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

See `extension/README.md` for extension setup, test commands, and launching an
Extension Development Host. Configure `iris.serverPath` to the absolute path
of `target/debug/iris-lsp` (or `iris-lsp.exe` on Windows). Configure
`iris.executablePath` to the Iris CLI executable to enable explicit VM runs.

Source is hosted at https://github.com/yuwenhuisama/iris-language-tools.
No Marketplace extension or packaged binary release is available yet.
