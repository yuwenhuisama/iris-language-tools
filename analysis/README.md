# iris-analysis

Pure shared semantic queries for Iris editor tooling. Dependencies are the sibling
builtin catalog, parser, syntax, and lexer crates. Analysis never opens files, maps
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
let hover = snapshot.hover(FileId(1), 28);
let signature = snapshot.signature_help(FileId(1), 28);
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
- `hover(file: FileId, byte: usize) -> Option<HoverInfo>` where
  `HoverInfo { span, signature, type_label, kind, owner, details, docs }`.
  The original `span: Span`, `signature: String`, and `type_label: Option<String>`
  remain available. Additive `kind: HoverKind` is the parser's declaration kind,
  `owner: Option<String>` is the graph-derived lexical owner, and
  `details: Vec<HoverDetail>` distinguishes `ReturnType(String)`,
  `ValueType(String)`, and `ParameterCategory(ParameterCategory)`.
  The span is the exact occurrence in the requested file, including an import
  alias, rather than the target declaration's span. Unlike navigation, hover
  accepts only positions inside the half-open occurrence, not immediately after
  a name. Invalid UTF-8 boundaries, protected text, ambiguity, and unsafe recovery
  positions return `None`.
- `signature_help(file: FileId, byte: usize) -> Option<SignatureHelpInfo>` where
  `SignatureHelpInfo { signatures: Vec<SignatureInfo>, active_signature: usize }`.
  `SignatureInfo { active_parameter: Option<usize>, label: String, parameters: Vec<SignatureParameterInfo>, docs }`
  contains a plain signature label and its parameter metadata. Each parameter has
  `label: Span` (UTF-8 byte offsets into the signature label), `name: String`,
  `category: ParameterCategory`, and `docs: Option<DocumentationInfo>`. Categories
  are `Positional`, `Rest`, `Keyword`, `KeywordRest`, and `Block`. An empty parameter
  list has no active parameter; an unmappable argument returns no help, rather than
  letting LSP default to zero. When multiple call candidates exist (such as built-in
  alternative call shapes or keyword variants), `signatures` carries valid candidates and
  `active_signature` selects the first compatible shape in catalog order.
- `DocumentationInfo { text: String, truncated: bool }` is plain untrusted text,
  attached by parser declaration identity or built-in catalog documentation, not
  interpreted Markdown. All `docs` fields use `Option<DocumentationInfo>`. Parameter
  docs remain absent unless attached to that parameter declaration.

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

String `split`, `chars`, `to_array`, and text `graphemes` preserve structural
`Array<String>` facts. Integer indexing yields `String?` (including out-of-bounds
nil), while Range indexing retains the array type. Immutable copies and explicit
`Array<Builtin>` annotations preserve element facts without parsing display labels.
Nullable elements do not expose definite String completions. Element inference
refuses unknown/Dynamic elements or indexes, shadowed annotations, reopened Array
or producer families, and incomplete inventory. A bounded same-file use scan
refuses mutations, member calls, escapes, constant exposure, mutable aliases, and captures; it is not
flow-sensitive and does not attempt general generic substitution.

Nominal declaration values remain class/module/contract objects, not instances of
their header types. Positional and keyword rest bindings have body types
`Array<T>` and `Hash<Symbol, V>` under CONTROL-C023. Copies can display these
container labels and known Array/Hash methods, but do not acquire the element
type's member surface. Their omitted signature annotations still show `Dynamic<Object>`.

Recovery preserves usable earlier facts only while the relevant recovery regions
are beyond the query/name. Incomplete members retain replacement anchors, and
completion marks parser recovery as incomplete. Protected comments and literal
regions never receive semantic completions.

Hover signatures preserve grammar-owned headers and original literal/default
token bytes without evaluating them or including declaration bodies. Nominal
headers show declaration identity, not an instance annotation. Method signatures
retain source visibility, surface, modifiers, generic parameters, and ordered
parameter channels, including discard parameters. Omitted visibility is made
explicit; omitted named-method parameter and return annotations are displayed as
`Dynamic<Object>`. Rest parameter signatures show their element annotations while
`type_label` reports their body container types. For methods, `type_label` is the
declared return annotation (or `Dynamic<Object>`), not an inferred callable type.
Written `typeof` remains in the signature but has no inferred type label.

Signature and type-label payloads together are bounded to 4096 UTF-8 bytes, with
at most 1024 bytes for the type label. Text is accumulated incrementally and
truncated at character boundaries with `... [truncated]`. Trivia is compacted
without rewriting literal token contents. Source documentation comes from parser
attachments, preserves paragraphs, and is independently bounded to 2048 UTF-8
bytes with the parser's `truncated` flag. Owner labels are bounded to 1024 bytes.

Hover and Signature Help use the same parser-owned signature and parameter slots
for source methods, and catalog call shapes for builtins, without reconstructing
parameter grammar. Signature Help labels are bounded to 4096 UTF-8 bytes; oversized
complete structures return `None`, never truncated labels with invalid parameter
ranges. The active slot uses only the selected call's parser-recorded separators.
The innermost enclosing call wins even when its callee is unknown. The cursor
must be after the opening token and no later than the byte before the closer, or
at the incomplete editor end. Keyword arguments map by exact keyword name and
then keyword-rest; positional arguments skip keyword/block channels and can repeat
a positional-rest slot. Built-in candidates present their distinct call shapes
and retain compatible shapes in catalog order based on supplied argument channels.
No splat, initializer-specific constructor, arbitrary inheritance, or closure
signature is synthesized. Cataloged fixed constructors remain available.
Incomplete-call assistance resolves only a clean callee prefix before recovery;
the other queries' recovery barriers remain unchanged.

## Builtin Hints and Catalog Integration

Editor queries resolve built-in members and types using the shared, inert
`iris-builtins` crate (`crates/iris-builtins` in the sibling `Iris-Language` tree).
Because `iris-analysis` depends directly on `iris-builtins`, checking out older
sibling revisions lacking this crate fails the build immediately rather than
silently degrading or relying on an uncommitted commit hash.

- **Receiver Resolution**: Known receiver types from literals (such as String,
  Array, Hash, Range, Bytes, ByteArray, Float32, Float64, Integer), unannotated
  immutable local bindings initialized to these literals, and typed annotations resolve to
  built-in member catalogs.
- **Nominal Values**: Immutable copies of known Class and Contract values retain
  their metadata surface. A value annotated with a user Contract is not the
  Contract object itself. Named services and Module source-name routes are not
  treated as first-class service objects. Reading an ordinary Object method does
  not infer the return type of calling that method.
- **Surface Distinctions**: Members are segregated into instance, class-side,
  service, global, and property surfaces. Class-side calls (e.g. `Float64.from_bits`
   or `Object.new`) never mix with instance methods (e.g. `(1.0).to_bits`). Services
  like `JSON` or `Unicode` route by qualified namespace and possess no instance
  receiver. Properties have zero-parameter call shapes without permitting arbitrary
  parentheses.
- **Positional Placeholders vs Keywords**: Catalog labels such as `arg1`, `arg2`,
  or `callback` are neutral positional placeholders. They are not true keyword argument
  names. Only explicitly evidenced keyword arguments (such as `by(step: ...)` or
  `JSON.decode(depth: ...)`) accept keyword call syntax.
- **Return Facts and Unknown Types**: Catalog return facts reflect observed runtime
  results, not formal declared annotations. Where returns vary by backend, depend
  on callbacks or elements, or represent dynamic values, return types remain omitted
  (unknown) rather than fabricated. Unknown returns do not offer fake navigation or
  speculative dynamic narrowing.
- **Refusal and Internal Exclusions**: Internal testing probes (e.g. `share_count`),
  fixtures (`NativeFixture`), and refusal-only routes (e.g. `File.read_text` or
  `FFI::Library.call`) are excluded from completion and hints.
- **Backend Availability**: Hover cards and signature help document backend
  availability (reference evaluator, register bytecode VM, or both) alongside
  audit evidence anchors, avoiding false parity claims.

Hosts must mark missing source inventory with
`with_incomplete_groups(impl IntoIterator<Item = GroupId>)`. Catalog fallback is
disabled for those groups because unseen files could shadow names or reopen
builtins; known source facts remain available. The server conservatively marks
all loaded groups when its workspace inventory is incomplete.

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
cargo run -p iris-analysis --example hover
cargo run -p iris-analysis --example signature_help
```

Integration tests import the public crate and use the real parser. The snapshot
scenario exercises all four features across a target-file signature edit while
proving the previous immutable snapshot retains its old result. Prefix tests
exercise every UTF-8 boundary of an incomplete representative source.
