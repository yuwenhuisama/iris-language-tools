# iris-analysis

Pure shared semantic queries for Iris editor tooling. The only dependencies are
the sibling parser, syntax, and lexer crates. Analysis never opens files, maps
URIs, launches processes, evaluates Iris, or communicates with an LSP client.

## Public API

```rust
use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput, Span};

let source = "module Main { let value = 1; value }";
let snapshot = AnalysisSnapshot::new([SourceInput {
    id: FileId(1),
    group: GroupId(1),
    text: source.into(),
}]);
let definitions = snapshot.definitions(FileId(1), 29);
let references = snapshot.references(FileId(1), 18, true);
let completions = snapshot.completions(FileId(1), 32);
let hints = snapshot.inlay_hints(FileId(1), Span { start: 0, end: source.len() });
```

- `SourceInput { id: FileId, text: Arc<str>, group: GroupId }`.
- `FileId(pub u32)` and `GroupId(pub u32)` are assigned by the coordinator.
- `AnalysisSnapshot::new(impl IntoIterator<Item = SourceInput>)` parses with
  `parse_editor`. Duplicate file IDs use the last supplied input.
- `definitions(file, byte) -> Vec<Target>` where
  `Target { file, span, name_span }` separates the declaration and selection range.
- `references(file, byte, include_declaration) -> Vec<Location>` where
  `Location { file, span }` contains exact, deduplicated name sites.
- `completions(file, byte) -> CompletionResult` where
  `CompletionResult { items, is_incomplete, allow_keywords }`, and each
  `CompletionItem { label: String, detail: Option<String>, kind, replace: Span }`.
- `CompletionKind` is `Variable`, `Constant`, `Parameter`, `TypeParameter`,
  `Class`, `Module`, `Contract`, `TypeAlias`, `Method`, or `Property`.
- `inlay_hints(file, range) -> Vec<TypeHint>` where
  `TypeHint { offset: usize, label: String }`. Labels include `: ` or ` -> `.

All coordinates are UTF-8 bytes; spans and hint request ranges are half-open.
Completion replacement ranges can extend past the cursor to replace an existing
partial selector. Navigation accepts a cursor immediately after a name.
Unknown files return empty results. Snapshots are immutable and independently
owned: build a new complete snapshot after an edit or project-input change.
The coordinator supplies manifest-selected sources in the same group and owns
file discovery, package identity, URI conversion, UTF-16 conversion, cancellation,
and ordinary keyword completion.

The coordinator may merge ordinary keywords only when `allow_keywords` is true.
Member/namespace, literal/comment, and unsafe recovery contexts deny keywords.
On a whole-document lexer failure, a clean lexer prefix bounded to 64 KiB may
permit keyword-only results before the failure; semantic links remain empty.

## Resolution Guarantees

Identity is `(FileId, declaration SyntaxId)`, not a name string. Syntax IDs are
snapshot-local. Source graph children, scopes, activation offsets, receiver links,
and exact sites drive analysis. Type-name sites are resolved too, without
double-counting the graph's parent/child copies. Union/intersection source name
groups are separated using the original bytes between recorded sites.

Locals activate after their initializer. Closures capture lexical declarations;
named methods do not capture outer body locals. Parameters and type parameters
obey their source scopes. Same-scope duplicates and duplicate nominal origins
are ambiguous, not an arbitrary first match. Cross-file queries stay inside the
host-assigned group. Explicit namespace imports and aliases resolve to their
target declaration. Qualified declarations such as `class Core::Thing {}` are
not exposed as unrelated unqualified `Thing` names.

Member queries use the known owner and instance/class/module surface. Private
and protected members are offered only inside the same lexical owner; subclass
authorization is not guessed. Conditional, open, implementation-qualified, and
uncertain composed surfaces are suppressed. An unknown or `Dynamic` receiver
never causes a same-selector search across unrelated owners.

Local hints cover literals (including Float32/Float64 and text/binary mutability),
written annotation copies, binding copies, non-generic known `.new()` construction,
and known written synchronous method return annotations. Written annotations are
never narrowed by their initializer. Omitted named-method parameter and return
annotations remain `Dynamic<Object>` under TYPES-C003; defaults and method bodies
never become inferred signature metadata. Return hint anchors come directly from
the parser's `return_hint_offset`.

Nominal declaration values remain class/module/contract objects, not instances of
their header types. Positional and keyword rest bindings have body types
`Array<T>` and `Hash<Symbol, V>` under CONTROL-C023. Copies can display these
container labels, but do not acquire the element type's member surface or guessed
built-in members. Their omitted signature annotations still show `Dynamic<Object>`.

Recovery preserves usable earlier facts only while the relevant recovery regions
are beyond the query/name. Incomplete members retain replacement anchors, and
completion marks parser recovery as incomplete. Protected comments and literal
regions never receive semantic completions.

## Conservative Limits

- A group ID does not encode a package ID. Dotted package imports are therefore
  recognized and suppressed, never reinterpreted as matching namespace suffixes.
  External package mapping requires a future explicit host-provided contract.
- Inheritance, mixin/contract composition, generic substitution, re-export
  facades, extension activation, dynamic mutation, and overload-like dispatch
  are not synthesized. Header flags suppress uncertain member surfaces.
- Incomplete `Namespace::` without a surviving name production is suppressed;
  qualified partial identifiers such as `Namespace::Th` are supported.
- Unsupported operators, branches, extraction patterns, closures' contextual
  signatures, async call results, and generic call results remain unknown.
- Stored accessor capabilities and runtime storage-sigil lookup are not inferred.
  Type aliases navigate by identity but are not expanded into member surfaces.
- This is semantic editor assistance, not a complete static checker. Queries do
  not execute programs or treat parser acceptance as proof of runtime validity.
- Query indexing is intentionally snapshot-local and scan-based. Large projects
  should run queries off the editor thread; incremental caches are not supplied.

## Verification

```sh
cargo test -p iris-analysis
cargo clippy -p iris-analysis --all-targets -- -D warnings
cargo fmt -p iris-analysis -- --check
```

Integration tests import the public crate and use the real parser. The snapshot
scenario exercises all four features across a target-file signature edit while
proving the previous immutable snapshot retains its old result. Prefix tests
exercise every UTF-8 boundary of an incomplete representative source.
