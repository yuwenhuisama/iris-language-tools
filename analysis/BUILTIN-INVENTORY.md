# Implemented Builtin Callable Inventory

Source audit: 2026-09-08, sibling `Iris-Language` working tree. This is implementation evidence for a static editor catalog, not a new language specification. No Iris program was executed. Include the successful public routes below; keep refusal-only, fixture, and spec-only routes out of ordinary completion. Preserve backend availability rather than pretending the evaluator and VM implement identical surfaces.

## Reading The Tables

- `E` = `crates/iris-eval/src/source_runtime.rs`; `K` = `crates/iris-runtime/src/kernel.rs`; `N` = `crates/iris-runtime/src/numeric.rs`.
- `V` = `crates/iris-vm/src/machine/stdlib.rs`; `VA`, `VH`, `VI`, `VJ` = its `stdlib/array.rs`, `hash_text.rs`, `iteration.rs`, `json.rs` submodules.
- `O` = `crates/iris-vm/src/machine/operations.rs`; `X` = `crates/iris-vm/src/machine/execute.rs`; `C` = `crates/iris-vm/src/compile/calls.rs`.
- All these paths are relative to [the audited language checkout](../../Iris-Language/). `E:4561` means line 4561 in that file, not a spec clause. Anchors are audit-time locations and may move.
- `I` = instance send; `S` = named service/namespace call; `C` in the **Surface** column = class-side call; `P` = member/property read; `B` = bare helper; `Op` = operator/index syntax. A slash separates available surfaces, not overloads.
- **`p1`, `p2`, `p3`, `p4`, and `cb` are explicitly neutral positional slots, not canonical parameter names.** They are suitable display placeholders only when marked as such. `cb` denotes one Closure argument; a trailing block is lowered into that argument. Callback input tuples below describe invocation order, not keyword names or formal generic signatures.
- Alternatives separated by `or` are exact arity alternatives. `()` means zero arguments. `tail*` means the implementation accepts an arbitrary additional tail; it is **not evidence of a declared rest parameter**. `unchecked` means the arm never checks the argument count; display only the documented/useful zero-argument form, not invented variadics.
- Only explicitly shown `name:` spellings have keyword evidence. Ordinary positional names in the spec do not by themselves make keyword calls legal. Do not publish defaults, formal types, overload dispatch, or optional keyword contracts inferred merely from Rust local variable names.
- Returns are **observed code-result evidence**, not declared Iris signature annotations. `element`, `callback result`, and `stored value` mean no fixed concrete return type is justified. Type-family notation such as `Array of String` is descriptive. Omitted authored annotations remain `Dynamic<Object>`; do not replace them with builtin evidence.
- Unless noted, rows have both E and VM implementations. `E only` means a successful source-evaluator route was found but no matching VM call route in the audited dispatch. A VM bare-member route does not establish a VM parenthesized-call route.
- For non-kernel rows citing `V:binary_send` (V:13-225 or V:2287), VM evidence is **operator syntax only**, not necessarily a parenthesized selector send. This specifically affects text/binary `+`, MutableString `<<`, identity-bearing default comparisons, and Iteration ordering. E explicitly routes these through sends where its corresponding arm exists. Do not flatten an `I/Op` row into unconditional VM dot-call support.

## Dispatch And Scope Rules

The kernel installs native slots on only Object, Nil, Bool, Integer, Float32, Float64, and String (`K:245`). The rest are payload-specific dispatch, not an inherited universal method list. `Text` is a Rust value variant whose Iris name is **String**, never an additional builtin receiver type (`E:12900`).

`E:6369` resolves source call forms, `E:7902` handles class/meta/value sends, and `E:10479` handles payload-specific values. VM calls lower through `C:175`, run through `X:2280`, then `V:354` and `O:100`. Index expressions have separate routes (`E:7441`, `O:295`); do not claim `.[](p1)` works merely because `receiver[p1]` does. `E:8734` sends zero arguments for non-Object bare members, but ordinary Object method reads produce BoundMethod values instead. VM member handling is separate (`X:989`).

Service names are source-name routing, not proof of first-class service Class objects. E protects most service routes with `undeclared_name`, but always routes `Reflection::*`; VM shadowing checks differ by service (`C:384`). Bare `using` and `print` should only be offered as fallback helpers when not shadowed. E checks declared bindings/main methods (`E:6494`); VM special-cases `using` before binding lookup (`C:206`) and `print` after it (`C:247`).

## Numeric And Root Protocols

| Receiver | Surface | Selector / Call Shape | Return Evidence | Runtime Path / Qualification |
| --- | --- | --- | --- | --- |
| Integer, Float32, Float64 | I/Op | `+(p1)`, `-(p1)`, `*(p1)` | Integer for two Integer operands; otherwise numeric common width | K:323, K:589; N binary operations |
| Integer, Float32, Float64 | I/Op | `/(p1)` | Float64 for Integer/Integer; otherwise float common width | K:599; N:100 |
| Integer, Float32, Float64 | I/Op | `**(p1)` | Integer for nonnegative Integer exponent on Integer; negative exponent gives Float64; float path preserves/promotes width | K:600; N:135 |
| Integer | I/Op | `div(p1)`, `mod(p1)`, `<<(p1)`, `>>(p1)`, `&(p1)`, `\|(p1)`, `^(p1)` | Integer; argument must be Integer | K:601, K:610, K:736; `div`/`mod` also named infix |
| Integer | I/Op | `~()` | Integer | K:605; arguments are actually unchecked, intended unary arity 0 |
| Integer, Float32, Float64 | I/Op | `negate()`; unary `-` lowers to `negate` | Same numeric family | K:651; arguments actually unchecked, intended unary arity 0 |
| Integer, Float32, Float64 | I | `mul_add(p1, p2)` | Float64 if any operand is Float64, else Float32, **including all-Integer inputs** | K:799; N:119, N:494. Integer slot is implemented, not just float slots |
| Float32, Float64 | I | `is_nan()`, `is_signaling_nan()`, `is_infinite()`, `is_finite()`, `is_normal()`, `is_subnormal()`, `is_zero()`, `sign_bit()` | Bool | K:658, K:844; exact arity 0, no `?` suffix |
| Float32, Float64 | I | `to_bits()` | Integer | K:666; width-specific unsigned bit range |
| Float32, Float64 Class objects | C | `from_bits(p1)` | Float32 or Float64 matching receiver | K:753; C:794; p1 Integer bit pattern; range checked |
| Float32, Float64 Class objects | C/P | `nan()`, `infinity()`; `.nan`, `.infinity` | Float32 or Float64 matching receiver | K:780; X:1360. Kernel also installs names on numeric instance revisions, but these bodies require a Class receiver |
| Numeric, Nil, Bool | I/Op | `==(p1)`, `!=(p1)`, `<(p1)`, `<=(p1)`, `>(p1)`, `>=(p1)` | Bool | K:619, K:689; E:11415 derived comparison route |
| Numeric, Nil, Bool | I/Op | `<=>(p1)` | Integer -1/0/1 or nil | K:645, K:689; unordered/non-numeric operands can give nil |
| Numeric, Nil, Bool, String | I | `hash()` | Integer on success; NaN fails | K:809; O:178. Hashability is conditional, not a promise for all floats |
| Default Object/value protocol | I | `to_bool()` | Bool; nil false, Bool itself, other default values true | K:815; E:8624. Class/payload interception can restrict direct reachability; authored overrides win where dispatched |
| Ordinary heap Object | I | `hash()` | Stable identity Integer | E:8638; V:748 |
| Ordinary heap Object | I/Op | `<=>(p1)`; `==(p1)`, `!=(p1)`, `<(p1)`, `<=(p1)`, `>(p1)`, `>=(p1)` | nil default ordering; Bool comparisons, identity/default or authored ordering | E:8649, E:11631; V:13; O:142 |
| Ordinary heap Object | I | `to_string()`, `inspect()` | String nominal-name rendering | E:12277; V:2450; fallback bodies render directly; do not claim inspect invokes an override of to_string |
| Any receiver in E | I | `respond_to?(p1)` | Bool | E:7973; p1 Symbol; only heap Object gets actual slot test, other kinds return false. E only |
| Identity-bearing receivers | Primitive/I/Op | `same?(p1)` | Bool, or IdentityError for unsupported/identity-less operands | E:7832, E:7939; C:869; O:212. Not an ordinary overridable builtin Method |
| Integer, Float32, Float64, Nil, Bool, Symbol | I | `to_string()` | String | E:4280; V:736, V:861 |

## Arrays, Hashes, Tuples, Ranges, Readonly Views

| Receiver | Surface | Selector / Call Shape | Return Evidence | Runtime Path / Qualification |
| --- | --- | --- | --- | --- |
| Array | I | `length()`, `count()` | Integer | E:11084, E:4624; VA:17 |
| Array | I | `empty?()` | Bool | E:11111; E only |
| Array | I | `to_array()`, `reverse()`, `sort()`, `uniq()`, `flatten()` | Fresh Array | E:4493, E:4644, E:4728; VA:20, VA:109, VA:159, VA:197 |
| Array | I | `map(cb)`, `select(cb)`, `reject(cb)` | Array; map contains callback results, select/reject preserve elements | E:4562; VA:32; callback `(element)` |
| Array | I | `each(cb)`, `each_with_index(cb)` | Receiver Array | E:4569; VA:39; callbacks `(element)` / `(element, index)` |
| Array | I | `find(cb)` | Element or nil | E:4615; VA:61; callback `(element)` |
| Array | I | `count(cb)` | Integer | E:4627; VA:76; callback `(element)`; separate alternative from `count()` |
| Array | I | `reduce(cb)` or `reduce(p1, cb)` | Accumulated callback result; empty without seed nil; empty with seed p1 | E:4598; VA:85; callback `(accumulator, element)` |
| Array | I | `sum()` | Accumulated `+` result, initially Integer 0 | E:4637; VA:101; do not force Integer result for arbitrary elements |
| Array | I | `min()`, `max()`, `first()`, `last()`, `pop()` | Element or nil | E:4644, E:4651; VA:108, VA:153 |
| Array | I | `push(p1)` | Receiver Array | E:4655; VA:110 |
| Array | I | `append(p1)`, `delete(p1)`, `insert(p1, p2)`, `clear()` | nil | E:198; VA:118; insert p1 Integer; delete removes first equal element; **append is not push's return alias** |
| Array | I | `join(p1)` | String | E:4660 accepts to_string-convertible separator; VA:154 requires String |
| Array | I | `include?(p1)` | Bool | E:4668; VA:164 |
| Array | I | `index_of(p1)` | Integer or nil | E:4677; VA:165 |
| Array | I | `concat(p1)` | Fresh Array | E:4688; VA:169; p1 Array |
| Array | I | `slice(p1, p2)` or `slice(p1)` | Fresh Array | E:4693; two Integers for start/count, one Range alternative **E only**; VA:174 implements two only |
| Array | I | `take(p1)`, `drop(p1)` | Fresh Array | E:4717; VA:185; nonnegative Integer count |
| Array | I | `all?(cb)`, `any?(cb)` | Bool | E:4749; VA:201; callback `(element)`; no zero-argument alternative |
| Array | I | `at(p1)` | Element or nil | E:4760; VA:214; p1 Integer |
| Array | I | `to_string()` | String | E:4765; VA:217 |
| Hash | I | `length()` | Integer | E:11089; VH:16 |
| Hash | I | `empty?()` | Bool | E:11114; E only |
| Hash | I | `keys()`, `values()`, `to_array()` | Array of keys / values / two-element Tuples | E:4499; VH:28, VH:116; no public ordering promise |
| Hash | I | `include?(p1)`, `has_key?(p1)` | Bool | E:4516; VH:42 |
| Hash | I | `fetch(p1)` | Stored value; absent raises KeyError | E:4413; VH:48; no implemented default/block alternative |
| Hash | I | `delete(p1)` | Removed value or nil | E:4421; VH:52 |
| Hash | I | `rehash()` or `rehash(cb)` | nil | E:4406, E:5615; VH:22, VH:202; merge callback `(kept_key, kept_value, incoming_key, incoming_value)` must return `(key, value)` Tuple |
| Hash | I | `each(cb)`, `each_with_iterator(cb)` | **E nil; VM receiver Hash** | E:4434 closes traversal with nil outcome; VH:60 returns receiver; callbacks `(key, value)` / `(key, value, iterator)` |
| Hash | I | `map(cb)` | Array of callback results | E:4522; VH:83; callback `(key, value)` |
| Hash | I | `select(cb)` | Hash | E:4529; VH:90; callback `(key, value)` |
| Hash | I | `merge(p1)` | Fresh Hash | E:4539; VH:104; p1 Hash |
| Tuple | I | `to_array()` | Array of elements | E:4461; V:681; no implemented `length()` or `iterator()` send found |
| Range | I | `by(p1)` or `by(step: p1)` | New Range | E:4375; V:704; p1 Integer, nonzero correctly directed step; exactly one supplied argument |
| Range | I/P | `start()`, `end()`, `inclusive_end?()` | Integer, Integer, Bool | E:11121; E only; no `step()` getter found |
| Range | I | `to_array()` | Array of Integer | E:4461; V:674 |
| ReadonlyArray | I | `length()` | Integer | E:11049; V:858 |
| ReadonlyArray | I | `empty?()` | Bool | E:11108; E only; no generic Array convenience-method inheritance |

### Index Surfaces

These arities describe index protocols, not automatically dot-call signatures. Source assignment-result facts must be kept separate from formal setter results.

| Receiver | Surface | Selector / Call Shape | Return Evidence | Runtime Path / Qualification |
| --- | --- | --- | --- | --- |
| Array | Op | `[]`: `[p1]`, Integer or Range | Element or nil / Array slice | E:7492, E:7531; O:297 |
| Hash | Op | `[]`: `[p1]` | Value or nil | E:7549; X index dispatch, O:316 |
| Tuple, ReadonlyArray | Op | `[]`: `[p1]`, Integer | Element or nil | E:7482, E:7531; O:317 |
| ReadonlyArray | Op | `[]`: `[p1]`, Range | Mutable Array copy | E:7492; E only |
| String | Op | `[]`: `[p1]`, Integer or Range | One-scalar String or nil / String slice | E:7510; O:374 |
| MutableString | Op | `[]`: `[p1]`, Integer or Range | VM one-scalar String or nil / String slice | O:374; E:7445 mistakenly enters byte extraction and refuses; do not claim backend parity |
| Bytes, ByteArray | Op | `[]`: `[p1]`, Integer or Range | Integer byte or nil / same-family slice | E:7445; O:336 |
| Array, Hash | Op | `[]=`: `[p1] = p2`, Integer index / arbitrary Hash key | Source expression returns RHS in both backends | E:7383, E:7565; X:960, O:403. E helper returns mutated container; VM helper returns stored value; spec says nil. Array Range write not implemented here |
| MutableString | Op | `[]=`: `[p1] = p2`, Integer or Range; p2 String/MutableString | Setter helper nil; E source assignment returns RHS | E:7591, E:7394; O:483; scalar replacement exactly one scalar |
| ByteArray | Op | `[]=`: `[p1] = p2`, Integer byte or Range binary replacement | Setter helper nil; E source assignment returns RHS | E:7630, E:7394; O:443; byte Integer 0..255, Range replacement Bytes/ByteArray |

## Text, Binary, Regex

| Receiver | Surface | Selector / Call Shape | Return Evidence | Runtime Path / Qualification |
| --- | --- | --- | --- | --- |
| String | I | `length()`, `byte_length()` | Integer scalar count / UTF-8 byte count | E:11096; VH:131, VH:159 |
| String | I | `empty?()` | Bool | E:11117; E only |
| String | I | `to_string()`, `inspect()` | String; self text / escaped reparsable literal | E:4294, E:4332; VH:132, VH:173 |
| String | I | `split(p1)` | Array of String | E:4295; VH:133; p1 String; no zero-arg/default separator |
| String | I | `trim()`, `downcase()` | String | E:4305, E:4160; VH:138, VH:145 |
| String | I | `upcase()` | String | E:4160; **E only**, absent from VH despite docs calling it pre-existing |
| String | I | `replace(p1, p2)` | String | E:4306; VH:139; both String; not MutableString's one-argument replace |
| String | I | `starts_with?(p1)`, `ends_with?(p1)`, `contains?(p1)` | Bool | E:4312; VH:142; p1 String |
| String | I | `chars()`, `to_array()` | Array of one-scalar String | E:4147, E:4321; VH:146, VH:163 |
| String | I | `to_symbol()` | Symbol | E:4326; VH:151 |
| String | I | `to_bytes()` | Bytes UTF-8 snapshot | E:4368; VH:155 |
| String | I/P | `bytes()` | Bytes snapshot | E:4232; E only, not a lazy view in current code |
| String | I/Op | `+(p1)` | String via direct String append or to_string conversion | E:3952; V:217; canonical spec parameter name `other` exists, not keyword evidence |
| MutableString | I | `length()`, `to_string()` | Integer scalar count / String snapshot | E:4192; O:128, V:836 |
| MutableString | I | `to_bytes()`, `to_array()` | Bytes / Array of one-scalar String | E:4193, E:4147; E only |
| MutableString | I | `upcase()`, `downcase()` | Fresh MutableString | E:4167; V:598 |
| MutableString | I | `upcase!()`, `downcase!()`, `clear()` | Receiver MutableString | E:4178, E:4216; V:598, V:620 |
| MutableString | I | `append(p1)`, `replace(p1)` | Receiver MutableString; conversion through text/to_string | E:4206, E:4220; V:587, V:632 |
| MutableString | I/Op | `<<(p1)`, `+(p1)` | `<<` receiver; `+` **E fresh MutableString, VM String** | E:4206; V:144; VM operator accepts String/MutableString only, E converts other values |
| MutableString | I/P | `bytes()` | Receiver MutableString in current representation | E:4228; V:627; its iterator yields scalars, so do not infer Iterable of Integer from this route |
| String, MutableString | I | `casefold()`, `nfc()`, `nfd()` | **String for either receiver** | E:4235; V:641; not fresh MutableString for these non-bang transforms |
| String, MutableString | I | `graphemes()` | Array of String | E:4265; V:641; eager, not spec's lazy view |
| Bytes, ByteArray | I | `length()`, `to_string()`, `to_bytes()`, `to_array()` | Integer / strict UTF-8 String / Bytes snapshot / Array of Integer byte | E:4358, E:4461, E:11068; V:684, V:820, V:842, O:122 |
| Bytes, ByteArray | I | `empty?()` | Bool | E:11078; E only |
| Bytes, ByteArray | I/Op | `+(p1)` | Same receiver family, fresh value | E:3967; V:192; p1 Bytes or ByteArray |
| ByteArray | I/Op | `append(p1)`, `<<(p1)` | Receiver ByteArray | E:3967; E only; binary operand only |
| String, MutableString | I/Op | `=~(p1)`, `!~(p1)` | Match or nil / Bool | E:4000; V:424; p1 Regex |
| Match | I/P | `text()`, `to_string()`, `regex()` | String / String / Regex | E:4048; V:548 |
| Match | I/P | `start()`, `end()`, `byte_start()`, `byte_end()` | Integer scalar/byte offsets | E:4054; V:551 |
| Match | I | `capture(p1)` | String or nil | E:4068; V:565; p1 one-based Integer capture or Symbol/String capture name |
| Regex | I/Op | `==(p1)`, `!=(p1)`, `hash()` | Bool / Bool / Integer | E:11155, E:11368; O:178, O:588; no Regex matching/construction convenience methods found |

## Iteration, Callable Values, Errors, Resources

| Receiver | Surface | Selector / Call Shape | Return Evidence | Runtime Path / Qualification |
| --- | --- | --- | --- | --- |
| Array, Hash, Range, Bytes, MutableString | I | `iterator()` | Cursor; Array/Range element, Hash Tuple pair, Bytes Integer, MutableString scalar String | E:10635; VI:36 |
| ReadonlyArray, ByteArray | I | `iterator()` | Cursor | E:10635, E:10703; E only for explicit send |
| ArrayIterator, HashIterator | I | `next()`, `close()` | Iteration signal / nil | E:10824, E:10957; VI:93 |
| ByteIterator | I | `next()`, `close()` | Iteration signal / nil | E:10757; E only |
| HashIterator | I | `remove_current()` | nil | E:10860; VI:124; state/concurrent-modification checks |
| Generator | I | `iterator()`, `next()`, `close()` | Self / Iteration / nil | E:10889; explicit E route; not a generic Iterator constructor |
| Iteration service | S | `yield(p1)` | Iteration yield carrying p1 | E:6404; C:339; canonical spec name `value` |
| Iteration service | P | `.done` | Unique done signal | E:6968; **not evidence for `done()`** |
| Iteration yield/done | I/P | `yield?()`, `done?()`, `value()` | Bool / Bool / payload; done.value raises IteratorStateError | E:11137; VI:102 |
| Iteration yield/done | I/Op | `<=>(p1)`, `==(p1)`, `!=(p1)`, `hash()` | Integer or nil / Bool / Bool / Integer if payload hash succeeds | E:11155, E:11282; V:2287, O:178; both reject nil payload comparison despite spec allowing it; VM ordering is operator route |
| Closure, BoundMethod | I | `call(...)` | Callable's declared/result value; arity comes from that callable | E:11001; X:2329; **no fixed builtin signature**. Bare BoundMethod invocation also exists; bare Closure application is refused |
| Method | I | `bind(p1)` | BoundMethod | E:10909; V:1118; p1 heap Object or Class, binding checked |
| Method | I/P | `selector()`, `owner()`, `visibility()` | Symbol or nil / Class or module Symbol or nil / Symbol | E:8349 unchecked; V:1042 exact 0; VM Module owner currently nil |
| Method | I/P | `parameters()`, `return_type()` | Array of type-name Symbols / type-name Symbol | E:8390 unchecked; V:1060 exact 0; source-body metadata only, not kernel signature discovery |
| Method | I/P | `source()` | Class-owned: Array `[package Symbol, revision Integer, commit Integer, status Symbol]`; module-owned: Symbol | E:8373 unchecked; V:1095 exact 0; not a typed SourceLocation |
| Method, Closure, BoundMethod, Task | I/P | `class_name()` | Symbol naming the kind | E:10601, E:10886; V:396. VM lacks Method `class_name` arm |
| ExceptionContext | I/P | `value()`, `cause()`, `suppressed()`, `re_raise_sites()`, `original_stack()`, `raise_location()`, `class_name()` | Raised value / cause context or nil / ReadonlyArray / ReadonlyArray / ReadonlyArray / SourceLocation or nil / Symbol | E:10515 (arity unchecked); V:783 exact 0; X:1064 property reads |
| ExceptionContext | I/P | `decoder()`, `offset()`, `expected()` | Symbol or nil / Integer or nil / Symbol or nil | E:10557 unchecked; VM **property only** X:1086 |
| ExceptionContext | I/P | `operation()`, `caller_package()`, `target_scope()`, `denial_origin()` | Symbol or nil | E:10544 unchecked; E only; diagnostic metadata, not fields on raised values |
| SourceLocation | I/P | `path()`, `line()`, `column()` | E Symbol / Integer / Integer; VM path String | E:10614 unchecked; X:1105 VM **property only** |
| StackFrame | I/P | `callable_name()`, `location()` | Symbol / SourceLocation-like value | E:10624 unchecked; E only |
| RaiseSite | I/P | `location()` | SourceLocation-like value | E:10627 unchecked; X:1100 VM **property only** |
| External native resource (extension-declared nominal type) | I | `close()`, `closed?()`, `==(p1)`, `same?(p1)` | nil / Bool / Bool / Bool | `source_runtime/native.rs:44`; O:106; `iris-native-host/src/resource.rs:24`; comparisons require ExternalResource |

`Task` has no implemented `wait`, `join`, `result`, `then`, `cancel`, or ordinary `await()` method here. `await` is syntax. `Closeable.close()` is a contract obligation, not an automatically available method on every value. Exception class names in `catchable_name` are diagnostics, not evidence of a builtin constructor or message catalog for each error.

## Class, Module, Type And Transformation Metadata

For E getters below marked `unchecked`, zero is the useful/source-documented arity but the runtime ignores extra arguments. Do not turn that implementation permissiveness into editor parameter slots.

| Receiver | Surface | Selector / Call Shape | Return Evidence | Runtime Path / Qualification |
| --- | --- | --- | --- | --- |
| Object Class; declared Class / closed Class | C | `new(...)` | New ordinary heap instance | E:7991, E:7756; X:2418; constructor shape from `initialize`; absent initialize accepts/ignores arguments (`iris-runtime/src/runtime.rs:199`). Prefer `Object.new()`; do not invent scalar/collection constructors |
| Class | C | `open(cb)` | Callback result | E:2461; C:635, X:3548; callback receives Class; closed/generic open refusal is separate |
| Class | C | `define_method(p1, cb)`, `define_property(p1, cb)` | nil | E:2559, E:2632; C:351, V:899; p1 Symbol; VM define_method requires literal Closure AST rather than arbitrary stored closure |
| Class | C | `alias_method(p1, p2)` | nil | `source_runtime/boundaries.rs:178`; V:976; two Symbol selectors |
| Class | C | `remove_method(p1)`, `undef_method(p1)` | nil | E:8000; V:991; p1 Symbol |
| Class | C | `add_module(p1)`, `remove_module(p1)` | nil | E:8095; V:930; p1 module-name Symbol |
| Class | C | `remove_contract(p1)` | nil only when not declared; declared conformance refused | E:8080; V:952; p1 Contract |
| Class | C | `set_superclass(p1)` | nil | E:8106, E:9916; direct E route; VM has Reflection::Class form, not direct counterpart |
| Class | C | `method(p1)` | Method or nil | E:8026; V:1007; p1 Symbol |
| Class, Module | C | `invoke(p1, p2, p3)` | Retained Method invocation result | E:8039, E:6553; p1 Method, p2 receiver, p3 Array; direct E routes; VM supports service forms |
| Module source name | C | `method(p1)` | Method or nil | E:6541; direct E route |
| Module source name | P | `.modules` | Array of module-name Symbols | E:6997; not evidence for `modules()` |
| Class | C/P | `properties()` | ReadonlyArray of Symbols | E:8126 unchecked; V:884 exact 0; X:1137 property |
| Class | C/P | `active_revision()`, `contracts()` | Integer / Array of Contracts | E:8246 unchecked; V:874, V:1023 exact 0 |
| Class | C/P | `name()`, `methods()`, `modules()`, `static_spine()`, `denied_capabilities()`, `decorators()`, `decorator_arguments()`, `decorator_phases()`, `type()` | Symbol or nil / ReadonlyArray Symbols / Array Symbols / Integer / Array Symbols / ReadonlyArray Symbols / nested ReadonlyArrays / ReadonlyArray Symbols / Type | E:8136-8416 unchecked; X:1114-1328 VM **properties only** for this group |
| Class | C/P | `package()`, `runtime_superclass()`, `mro()`, `ancestors()`, `meta_capabilities()` | Symbol / Class or nil / Array of Classes / same / Array of denied-capability Symbols | E:8133, E:8145, E:8335 unchecked; E:10062 filters Modules out of mro/ancestors; E only |
| Nominal Type | I/P | `type()`, `kind()`, `package()`, `hash()`, `arguments()`, `members()` | Self / Symbol `nominal` / Symbol / Integer / Array of Types / Array of selector Symbols | E:8450, E:8481 unchecked; V:1141 exact 0 |
| Nominal Type | I | `subtype?(p1)`, `assignable?(p1)` | Bool | E:8485; V:1236; p1 nominal Type |
| Composed Type | I/P | `type()`, `kind()`, `members()` | Self / Symbol `never`, `union`, `intersection` / Array of Type atoms | E:8450 unchecked; V:1184, V:1216 exact 0; not nominal members' selector list |
| Contract | I/P | `parents()` | Array of Contracts | E:8304 unchecked; E only |
| Contract | I | `view(p1)` | ContractView around Class | E:8272; p1 Class; E only |
| ContractView around Class | I | `respond_to_contract?(p1)` | Bool | E:8281; p1 Symbol; E only |
| Contract, ContractView | I | `hash()` | Integer | E:11231, E:8429; V:369, V:800; view composes receiver hash with Contract identity |
| Transformation value / special `Transformation` name | I/P | `empty()`, `kind()` | Empty Transformation / Symbol `class` | E:7950 unchecked; `source_method.rs:241`; E only |
| Transformation | I | `add_method(p1, cb)` | New Transformation with staged entry | E:7960; p1 Symbol; E only; canonical C125 positional names `selector`, `body` |

`Class.rollback` is a partial artifact-validation route, listed below rather than advertised as a successful rollback implementation. Generic Class metadata forwarding in E can erase closed arguments (`E:7908`); use source `.type` handling (`E:7019`) rather than inferring `Box<T>.type()` from the open Class row.

## Service And Bare Calls

| Receiver | Surface | Selector / Call Shape | Return Evidence | Runtime Path / Qualification |
| --- | --- | --- | --- | --- |
| None | B | `print(p1, ...)` with **0 or more** supplied arguments | nil | E:6506; C:247; renders each via text conversion; docs/authored-stdlib.md:28 |
| None | B | `using(p1, cb)` or `using(p1) { ... }` | Callback result, or raised cleanup/body failure | E:8809; C:206; callback receives **zero** arguments; canonical spec `resource`, block channel, not invented keyword |
| None | B | `Integer(p1)` | Integer | K:497; E:7694; X:2086; p1 Integer only; not String parser |
| None | B | `Float64(p1)` | Float64 | K:772; E:7694 accepts Integer/Float32/Float64; X:2089 VM ordinary form accepts **Integer only** |
| None | Special spelling | `Float64(-Infinity)` | Negative Float64 infinity | `iris-eval/src/lib.rs:1270`; C:277; not evidence that bare `Infinity` exists; source-runtime path may resolve differently |
| Unicode | S | `version()` | String table version | E:9159 unchecked; C:535 exact 0 |
| Encoding::UTF_8, Encoding::UTF_16LE, Encoding::UTF_16BE, Encoding::Latin_1 | S | `decode(p1, tail*)`; useful alternatives `decode(p1)` / `decode(p1, errors: p2)` | String | E:8979; C:389; V:1981; p1 Bytes/ByteArray; only Symbol `replace` activates lossy branch; strict otherwise; no `ignore` implementation |
| JSON | S | `encode(p1, tail*)`; useful `encode(p1)` / `encode(p1, canonical: p2)` | String | E:9163; C:598; VJ:65; Bool true activates canonical ordering; serializable representation required for nominal Object |
| JSON | S | `decode(p1, tail*)`; useful `decode(p1)` / `decode(p1, depth: p2)` | JSON value: nil/Bool/Integer/String/Array/Hash | E:9178; VJ:27; p1 String/Bytes/ByteArray; direct `depth:` Integer, **not** spec example `limits: { depth: ... }`; no arbitrary nominal inference |
| IrisValue | S | `encode(p1, tail*)` | Validated serializable representation, **not Bytes** | E:9086; C:504; V:1460; implementation is eligibility/representation adapter, not wire encoder |
| IrisValue | S | `decode(p1, tail*)`; useful `decode(p1)` / `decode(p1, element_limit: p2)` | Hash stream payload or nominal factory result | E:9097; V:1487; p1 Hash with magic/format_version; actual fallback limit 1024 is code behavior, not canonical signature default |
| FFI | S | `open(p1, tail*)`; useful `open(p1)` / `open(p1, declarations: p2)` | FFI::Library metadata handle | E:9228; C:488; V:1794; E also accepts positional Hash in slot 2; VM reads keyword only. No actual binary load occurs here |
| FFI::Library | I | `bind(p1, p2, tail*)` | Library value retaining same identity with new binding metadata | E:4086; V:1911; p1 Symbol/String; p2 signature Hash; no native invocation |
| FFI::Library | I | `signature(p1)`, `bound?(p1)`, `class_name()` | Stored signature or nil / Bool / **String** `FFI::Library` | E:4107, E:10589; V:1929; p1 Symbol/String |
| Host | S | `run(p1)` | Task's result or re-raised task failure | E:9387; C:577; p1 Task; unavailable from async/closure/open contexts; host drive surface, not Task wait |
| Gate | S | `new()`; `complete(p1)` or `complete(p1, p2)` | Gate / nil | E:8846 (new count unchecked), E:8853; C:545 exact 0 for new; completion p1 Gate, omitted p2 becomes nil |
| Diagnostics | S | `discarded_contexts()`, `unobserved_failures()` | Array of discarded payloads / Array of diagnostic Tuples | E:8830 unchecked; C:458, C:589 exact 0; unobserved tuple contains marker, Task, captured value/context, state |
| Revision | S | `subscribe(cb)` or `subscribe(cb, p2)` | nil | E:8872; X:3364; p2 capacity Integer in E (VM accepts other value as unbounded); callback `(event)` |
| Revision | S | `flush()` | Tuple `(status Symbol, delivered Array, undelivered Integer, errors Array)` | E:5119, E:8892 unchecked; X:3386 exact 0; status delivered/incomplete |
| Revision | S | `shutdown()`, `event_errors()` | nil / Array | E:8895 unchecked; X:3386, X:3484 exact 0 |
| RevisionHistory | S | `events(p1, p2)`, `recover(p1, p2)` | Array of Integer commit IDs | E:8904, E:8939; X:3488, X:3516; two Integer inclusive bounds; not full Revision objects; recover needs configured sink |
| RevisionHistory | S / host adapter | `configure_sink()` or `configure_sink(p1)` | nil | E:8927; X:3507; argument ignored, only an in-memory history copy is configured; do not advertise persistent storage semantics |
| Reflection::Object | S | `list_ivars(p1)`, `get_ivar(p1, p2)`, `set_ivar(p1, p2, p3)`, `remove_ivar(p1, p2)` | Array Symbols / stored value / stored value / removed value | E:9574, E:9750; p1 Object/Class, p2 Symbol; VM C:734/X:3281 only get/set subset, primarily Object target; missing remove raises |
| Reflection::Class, Reflection::Module | S | `method(p1, p2)` | Method or nil for Class; E Module missing raises UnsupportedConstruct | E:9605, E:9834; X:2918, X:3183; p1 Class/module Symbol, p2 Symbol selector |
| Reflection::Class, Reflection::Module | S | `invoke(p1, p2, p3)` | Retained Method result | E:9611; X:2938; p1 Method, p2 receiver, p3 Array; canonical spec names `method`, `receiver`, `args` |
| Reflection::Class | S | `remove_module(p1, p2)`, `remove_contract(p1, p2)`, `set_superclass(p1, p2)` | nil when permitted | E:9631; X:2895; p1 Class; p2 module Symbol / Contract / Class |
| Reflection::Class | S | `properties(p1)`, `revision(p1)` | ReadonlyArray Symbols / Hash with Symbol keys `number`, `commit_id` | E:9696, E:9728; X:3150; p1 Class |
| Reflection::Class | S | `ancestors(p1)` | Array of Classes, Modules filtered out | E:9719, E:10062; E only; p1 Class |
| Reflection::Class | S | `define_method(p1, p2, cb)`, `define_property(p1, p2, cb)` | nil | E:9734; p1 Class, p2 Symbol; VM C:351 define_method only, closure-literal restriction |
| Reflection::Contract | S | `requirement(p1, p2)` | Hash with `:return_type` reified value, or nil when absent | E:9684, E:2100; X:3199; p1 Contract including generic arguments, p2 Symbol; **not direct `Contract.requirement`** |
| Reflection::Package | S | `identity()`, `version()`, `dependencies()` | Array `[package Symbol, major Integer]` / Symbol or nil / ReadonlyArray of dependency Arrays | E:9553 unchecked; E only |
| Reflection::Package | S | `module_status(p1)` | Symbol `failed`, `initialized`, or `absent` | E:9415; exactly one Symbol; E only |
| Reflection::Package | S / partial host adapter | `reload()` or `reload(p1)` | Integer count of cleared failed-module marks | E:9434; p1 ignored, no reload IO; E only |
| Reflection::Package | S / partial host adapter | `load(p1)` or `load(p1, p2)` | Symbol handle to already-prelinked module | E:9442; p1 Symbol, p2 ignored, no package fetch; E only; **not `Package.load`** |
| Reflection::Package | S / partial host adapter | `upgrade(p1)` | Upgrade hook result, or nil if no hook | E:9488; p1 target-version Symbol; E only; current implementation's version/hook transaction, not a complete package manager |
| Package | S / validation adapter | `validate(p1, tail*)` | Symbol `validated`, or core-claim diagnostic | E:9067; C:468, X:662; recognizes `core_abi:` and `replaces_core_regex_literals:` Bool flags; p1 ignored, not full manifest validation |

Named options above are discovered by scanning an arbitrary argument tail. Unknown/extra tail entries are often ignored. For editor signature help expose only the named useful alternatives with evidence, not a fictitious complete keyword schema. No keyword inference is warranted for positional `Revision.subscribe`, `Gate.complete`, or reflective calls.

## Equality And Hash Coverage Beyond Numeric

Do not attach all Object methods to every Rust payload solely because kernel class lookup falls back to Object.

| Receiver Family | Successful Selector Evidence | Qualification / Path |
| --- | --- | --- |
| String, Symbol | `==(p1)`, `!=(p1)`, `hash()` | E:11155, E:11379; String K:281; Symbol VM same-family comparison O:541 |
| Bytes, ByteArray; String, MutableString | `==(p1)`, `!=(p1)` | Cross mutable/immutable same-content comparison E:11240, E:11261; O:544. Bytes hash succeeds; mutable hashes refuse |
| Tuple | `==(p1)`, `!=(p1)`, `hash()` | E:11195, E:11389; O:570; hash conditional on every element |
| Range | `hash()` E+VM; `==(p1)`, `!=(p1)` VM | E:11155; O:555; E's source binary route sends the selector but its value-send table has no successful Range equality arm |
| ArrayIterator, HashIterator, ByteIterator | `==(p1)`, `!=(p1)`, `hash()` | E:11327; V:2497/O:604; identity based |
| Closure, BoundMethod, ExceptionContext | `==(p1)`, `!=(p1)` | E:11396; V:2497/O:560; ExceptionContext hash E:11223, O:617; Closure hash VM O:612 only |
| Task, Gate, Generator | `==`, `!=`, `hash()` VM | V:2497, O:604; E has `same?` identity route but no equivalent direct hash/equality success arm identified |
| Class, Method | `==`, `!=` VM; `same?` E+VM | O:142; E:7832; do not infer Class/Method hash from nominal **Type** hash |
| Contract | `hash()` E+VM; `==`, `!=` VM | E:11231; O:591 |
| FFI::Library | `==(p1)` E+VM; `!=(p1)` E | E:11355; V:1964; signature handle identity, not path equality |

## Refusal-Only, Internal, Or Not Implemented As Advertised

| Receiver / Route | Exact Shape Or Recognized Selector | Classification And Evidence |
| --- | --- | --- |
| Array | `share_count()` | **Internal conformance probe**, Integer; E:11061, VA:22 explicitly exclude it from program surface. Do not offer ordinary completion |
| NativeFixture | `compact_gc()`, `concurrently_replace(p1, p2)`, `resource()`, `raise(p1)` | **Internal fixtures**; E:9296, C:439, V:2141. Return moved-count Integer / nil / NativeResource / raise. E resource arity unchecked; VM zero. Not public stdlib |
| NativeResource from NativeFixture | `close()`, `releases()`, `class_name()` | Fixture resource surface: nil / Integer / Symbol `FFI::Resource`; E:10595, E:10935, V:531. Keep separate from real ExternalResource routes |
| RevisionHistory | `prune(p1)` | **Fixture control**, exactly one Integer; nil; E:8961, X:3535 explicitly used to induce unavailable history |
| Class | `rollback(...)` | **Partial artifact validator**, E:8058 never consumes arguments, checks configured artifact/digest/spine and returns digest Symbol. Does not reconstruct/publish rollback. No honest fixed implemented parameter contract; spec intends `rollback(target)` |
| FFI::Library | `call(p1, tail*)` | **Always refuses**: unbound symbol -> UnboundNativeSymbol; bound symbol -> UnsupportedConstruct (or grant failure in E). E:4128, V:1952. Do not advertise as working invocation |
| File | `read_text(p1, tail*)` | **Always refuses**: missing/host-default `encoding:` -> ENCODING_EXPLICIT_REQUIRED; explicitly selected -> UnsupportedConstruct. E:9038, C:422 |
| Encoding | `default(...)` | **Always refuses**, EncodingSelectionError; E:9156, C:414; no meaningful successful arity |
| Reflection::Class | `reactivate(p1)` or `reactivate(p1, p2)` | **Always refuses**, validated Class then MetaTransactionSuspension; E:9666. VM accepts Class plus arbitrary tail but also refuses X:2886 |
| Method | `call(...)`, bare application | **Not ordinary callable**; E:7714, C:331 explicitly declines known Method.call. Use bind then BoundMethod.call or Reflection invoke |
| Closure | Bare `closure(...)` | Refused by E:7693 and VM BareCall; `.call(...)` is supported |
| Array, MutableString, ByteArray | `hash()` | **Implemented refusal**, InvalidKeyError; E:11214; O:178. No successful Integer hint |
| Hash, Match, ReadonlyArray and other unhashable payloads | `hash()` | VM public_hash failure -> InvalidKeyError (`O:198`); E may instead report missing selector. Not evidence of successful hashing |
| ReadonlyArray | `append`, `push`, `delete`, `clear`, `insert`, `[]=`, `reverse!`, `sort!` | Explicit ReadonlyMutation rejection E:10575, V:759; do not list these as mutable methods |
| ExceptionContext | Setters (E specifically `value=`, `cause=`, `suppressed=`; VM any name ending `=`) | ReadonlyProperty refusal E:10519, V:777; no writable properties |
| Integer/String and other kernel Class objects | Generic `.new(...)` fallback | Can allocate an **ordinary heap Object tagged with that Class**, not an intrinsic scalar value (`E:7991`, runtime construct); do not catalog `Integer.new` as Integer conversion or `String.new` as text construction |
| Array/Hash/Bytes/ByteArray/MutableString/Tuple/Regex/Task Class names | `.new(...)`, including generic collection construction | No builtin intrinsic constructors found in kernel/source builtin-name map (`source_method.rs:208`, K:497). Literal support or spec examples are not constructor evidence |
| Array | `size`, `+`, `sort!`, `reverse!`; zero-arg `join`, `all?`, `any?`; default/block `fetch` on Hash | Not implemented convenience alternatives. `size` and `+` explicitly pinned absent in docs/authored-stdlib.md:15 |
| Hash | `clear()` | Spec-listed but no successful dispatch arm in E collection_mutation or VH |
| ByteArray | `clear()`, `replace(...)` | Spec-listed but no successful dispatch arm |
| String/MutableString | `normalize`, `normalize!`, `casefold!`; String `iterator()` | Spec surface not backed by these callable routes. Implemented normalization spellings are `nfc`/`nfd`; String default traversal specification is not evidence of an explicit successful iterator send |
| Tuple/ReadonlyArray | Array convenience methods including `map`, `to_string`, ReadonlyArray `to_array` | No blanket inherited convenience catalog; only rows explicitly listed above |
| Range | `step()`, `length()`, fully-open construction API | No implemented getter/construction arm; `by` and literal tokens do not imply them |
| Contract | `requirement(p1)` | Spec/vector spelling is not the implemented call entry; use Reflection::Contract.requirement |
| Class/Module/Method metadata | `signature`, Method `package`, unlisted Class/Module reflection names | Spec minimal metadata table is broader than implemented routes; do not manufacture getters |
| Package | `load`, `reload`, `upgrade` | E routes are **Reflection::Package**, not Package; Package dispatch implements validate only |
| None | `type_of`, `type_and_value`, `same?` as bare helper, `puts`, `len`, `Float32(p1)` | No builtin bare-helper implementation found; spec fixture vocabulary and familiar-language names are not runtime contracts. `typeof` is syntax |
| Plan / decorator callbacks | `Plan.empty`, `plan`, `transform` | No builtin Plan object in source builtin map. User-declared decorator callbacks/protocol obligations are not universally installed callable methods |
| Serializable nominal objects | `serialize()`, Class `deserialize(p1)` | Runtime-invoked **user hooks**, not installed builtins: E:5338/E:5372, V:1460/V:1530. Require declared Serializable participation. Do not substitute spec example `to_serializable()` for actual dispatched `serialize` |
| Array, Hash, Match | `==(p1)`, `!=(p1)` | Spec/value representation equality is not a successful builtin send here: E has no matching value-send arm, and VM O:529 excludes these families. Container internals comparing Rust Values do not establish a public callable |
| HostABI / C ABI / extension fixture exports | Host function names, Native* fixture modules | Not general editor builtins. Extension-defined functions must come from their own declared metadata; C ABI exports are not Iris selectors |

## Catalog Implementation Rules And Audit Checks

1. Key descriptors by **receiver family + surface + selector**, not selector alone. `append`, `replace`, `class_name`, `members`, and `each` already prove why this is necessary.
2. Store exact alternative argument layouts independently of result evidence. Parameter slots in this document are explicitly neutral unless a canonical name is separately cited. Callback shape is useful evidence but does not license a fabricated `Block<S>` annotation or keyword/default.
3. Store `implemented`, `partial adapter`, `internal fixture`, `refusal-only`, and backend-specific availability separately. A successful metadata adapter is not proof of real file IO, FFI execution, package loading, wire serialization, or persistent audit storage.
4. No source declaration should be overwritten by catalog fallback; no runtime execution is needed for discovery. Do not expand a receiver's catalog using `respond_to?` at analysis time.
5. Use real implementation alternatives: Array slice has a one-Range **E-only** alternative, reduce/count/rehash have multiple shapes, Hash each has divergent returns, and byte/text units differ. Do not repair those differences in the catalog by presenting spec-only behavior as implemented.

Cross-checked source anchors: kernel selector source map + installation + invocation (`K:55`, `K:245`, `K:589`); E callable router, collection/authored handlers, class/meta handlers, service switch, value handlers and index boundaries; VM compiler special cases, authored handlers and each stdlib submodule, low-level operations, call/member/reflection/revision execution. Relevant spec checks: RUNTIME C061/C094/C113/C118; CONTROL C015/C075/C076; COLLECTIONS C011-C040/C044-C075/C083; ASYNC C015/C030-C033/C046-C055; META C097/C100/C118-C126; LIBRARY C009-C028. Authored convenience authority: [docs/authored-stdlib.md](../../Iris-Language/docs/authored-stdlib.md).

Verification is static source evidence only, intentionally: no user scripts, code changes, builds, commits, or runtime probes were run for this document. The completed tables were manually read and checked against the source dispatch branches and return expressions. Markdown LSP diagnostics were attempted but no `.md` server is configured. Subagent fan-out was unavailable at the delegated depth, so the dispatcher cross-check was performed directly.
