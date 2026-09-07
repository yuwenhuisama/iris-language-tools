# Iris v1 Official Style Guide

This document is the official style guide for Iris v1 source code. It defines conventions for indentation, layout, whitespace, declarations, comments, and literals.

Iris syntax is specified across the 14 formal specification chapters under `spec/iris-v1/`. This style guide does not alter or supersede any formal specification rule. Instead, it defines normative conventions that produce clean, uniform, and idiomatic Iris code.

## 1. Status And Tooling Boundary

This guide establishes conventions for Iris codebases and upcoming developer tooling.

### 1.1 Normative Style Versus Grammar

Grammar dictates what the Iris parser accepts or rejects. Style dictates how readable and maintainable programs are formatted within those grammar rules:
* Semicolons: Grammar allows an optional trailing semicolon after an expression statement. Idiomatic Iris style avoids semicolons except where required by syntax.
* Line limits: Grammar accepts single lines of arbitrary length. Idiomatic style keeps lines within a 120-column soft limit.
* Indentation: Grammar allows arbitrary whitespace between tokens outside literal fences. Idiomatic style requires consistent 2-space indentation.

### 1.2 Current Formatter Coverage

This document defines the target style, not a claim of complete implementation.
Refer to the formatter README and QA report for the current implementation's
supported constructs, safety refusals, and verification evidence.

## 2. File Organization And Encoding

Every Iris source file must observe the following physical encoding and layout rules:

1. Encoding: Strict UTF-8 without byte order mark (BOM).
2. Line Endings: Unix line feeds (`\n`, LF) outside protected literal content. Do not normalize a literal's internal line endings when that would change its value.
3. Trailing Line: Every nonempty source file must terminate with exactly one final newline. An empty file remains empty.
4. Exterior Whitespace: No trailing whitespace at the end of lines. Blank lines must contain zero spaces or tabs.
5. Shebang: If present, a shebang comment (`#!/usr/bin/env iris`) must appear on the very first physical line.

## 3. Indentation And Line Length

* Indentation Unit: Exactly two spaces per indentation level (`  `). Never use tab characters (`\t`) for indentation or alignment.
* Soft Line Limit: 120 columns. Break code at safe syntactic boundaries; indivisible tokens and protected literal/comment text may exceed the limit. Do not reflow comments automatically.
* Continuation Indentation: When an expression or parameter list wraps across multiple lines, indent continuation lines by exactly two spaces (`+2`) relative to the enclosing statement or block level.
* No Visual Alignment: Do not align tokens vertically across multiple lines using spaces. Vertical alignment creates fragile diffs and churn.

```iris
// Good: continuation indented by two spaces, no vertical alignment
let result = compute_primary_metric(
  alpha_parameter,
  beta_parameter,
  gamma_parameter,
)

// Bad: aligned to opening parenthesis
let result = compute_primary_metric(alpha_parameter,
                                    beta_parameter,
                                    gamma_parameter)
```

## 4. Declarations And Braces

### 4.1 Opening Braces

Opening braces (`{`) must remain on the same line as the declaration header, loop header, or conditional keyword. Never place an opening brace on its own line.

```iris
// Good
class SessionManager {
  fun active?() -> Bool {
    @running
  }
}

// Bad
class SessionManager
{
  fun active?() -> Bool
  {
    @running
  }
}
```

### 4.2 Methods And Functions

All named methods and functions (`fun`, `module fun`, `class fun`) must always use multiline blocks with the closing brace on a fresh line, even when the body contains only a single short expression.

```iris
// Good
fun identity(value: Integer) -> Integer {
  value
}

// Bad: single-line method body
fun identity(value: Integer) -> Integer { value }
```

### 4.3 Closures

Closures allow two distinct forms:
1. Single-Line Closures: Permitted only for simple expressions with no comments, provided the entire closure fits within the 120-character line limit. A single-line closure requires a semicolon after the parameter and return type header per grammar.
2. Multiline Closures: Required whenever the body spans multiple statements, contains comments, or exceeds the line limit. Retain the header's separating semicolon in the recommended layout; its closing brace sits on its own line.

```iris
// Good: compact single-line closure
let doubled = numbers.map({ |n: Integer| -> Integer; n * 2 })

// Good: multiline closure for non-trivial logic
let processed = items.map({ |item: Item| -> Result;
  let transformed = item.normalize()
  transformed.compute()
})

// Bad: multiline closure cramming statements onto one line
let bad = items.map({ |item: Item| -> Result; let t = item.normalize(); t.compute() })
```

### 4.4 Conditionals, Match, And Exception Blocks

Control structures follow clear brace placement conventions:
* `else` begins on a fresh newline after the preceding closing brace (`}`).
* `catch` and `finally` attach to the same line as the preceding closing brace (`} catch error {`, `} finally {`), unless an intervening comment requires a line break.

```iris
// Good: 'else' starts on a new line
if ready {
  start_worker()
}
else {
  queue_task()
}

// Good: 'catch' and 'finally' stay on the same line as the closing brace
try {
  database.commit()
} catch error: IOError {
  logger.error(error.message)
} finally {
  database.close()
}

// Good: match construct
match status {
  :active => handle_active()
  :idle => wait_next()
  _ => handle_unknown()
}
```

## 5. Statements And Semicolons

1. No Semicolons on Ordinary Statements: Semicolons are optional after ordinary statements. Do not write trailing semicolons on variable declarations, assignments, expression statements, or calls.
2. One Statement Per Line: Write one statement per line. Do not combine multiple statements onto a single line separated by semicolons.
3. Grammar Exceptions: Use semicolons only where mandated by Iris grammar:
    * Closure header separators: `{ |params| -> ReturnType; body }`, also retained in multiline closures by convention.
    * Stored property shorthand accessor blocks: `property name: Type { public get; private set; }`.

```iris
// Good: clean lines without semicolons
let alpha = 10
let beta = 20
let total = alpha + beta

// Bad: unnecessary trailing semicolons
let alpha = 10;
let beta = 20;

// Good: property accessor block requiring semicolons
property name: String { public get; private set; }
```

## 6. Collections, Lists, And Trailing Commas

Iris allows trailing commas in multiline list productions. Conventions depend on layout:

### 6.1 Multiline Lists

When argument lists, parameter lists, array literals, hash literals, or tuple literals break across lines, place each item on its own line and append a trailing comma after every element, provided the grammar production permits it.

```iris
// Good: multiline array with trailing comma
let configured_ports = [
  8080,
  8443,
  9000,
]

// Good: multiline hash literal
let status_codes = %{
  :ok: 200,
  :not_found: 404,
  :server_error: 500,
}
```

### 6.2 Inline Lists

Do not add unnecessary trailing commas to single-line lists or call argument lists.

```iris
// Good
let coordinates = [10, 20, 30]
let origin = (0, 0)

// Bad
let coordinates = [10, 20, 30,]
```

### 6.3 Preserving Singleton Tuples

In Iris, `(value,)` denotes a 1-element tuple. A parenthesized expression without a comma, such as `(value)`, is an ordinary grouped expression. Never remove the trailing comma from a single-element tuple literal.

```iris
// Good: singleton tuple requires the trailing comma
let single_item: Tuple<Integer> = (42,)

// Warning: grouped expression, not a tuple
let grouped: Integer = (42)
```

## 7. Spacing And Operators

### 7.1 Binary Operators, Assignments, And Arrows

Surround binary arithmetic, comparison, logical operators, assignment operators, and arrows with exactly one space on each side:
* Binary operators: `+`, `-`, `*`, `/`, `**`, `==`, `!=`, `<`, `>`, `<=`, `>=`, `<=>`, `=~`, `!~`, `&&`, `||`, `&`, `|`, `^`.
* Assignments: `=`, `+=`, `-=`, `*=`, `/=`, `**=`, `&=`, `|=`, `^=`, `<<=`, `>>=`, `&&=`, `||=`.
* Arrows: `->` (return type) and `=>` (match arm).

```iris
// Good
let sum = first + second
let valid = count > 0 && status == :ready
mut total = 0
total += 15

// Bad: missing spaces
let sum=first+second
```

### 7.2 Unary And Prefix/Suffix Operators

Keep unary prefix and suffix operators tight against their operands:
* Prefix operators: `!expr`, `-number`, `+number`, `~mask`.
* Range operators: `1 ..= 10`, `0 ..< length` (space around range operators).
* Identifiers with predicate/mutation suffixes: `ready?`, `save!`.
* Raw ivars and storage sigils: `@name`, `@@shared_count`, `$global_registry`.

### 7.3 Member Access, Qualified Names, And Contract Views

Never place whitespace around member access dots, double-colon path qualifiers, or contract view dots:
* Member access: `receiver.method()`.
* Path and type qualification: `Package::Module::Class`.
* Contract view dispatch: `view..method()`.

### 7.4 Generics

Generic angle brackets must stay tight against the enclosed types and the identifier:
* Type parameters: `Map<String, Integer>`, `Array<T>`.
* No space between the outer name and the opening angle bracket: `Box<T>`, not `Box <T>`.
* No space inside the outer angle brackets: `Box<T>`, not `Box< T >`.

### 7.5 Colons And Commas

* Colons: Keep the colon tight against the preceding token and put exactly one space after it: `name: Type`, `key: value`.
* Commas: Keep the comma tight against the preceding expression and put exactly one space after it: `[a, b, c]`, `(x, y)`.

```iris
// Good
fun process(identifier: String, options: Hash<Symbol, Object>) -> Bool {
  @cache.store(identifier, options)
  true
}

// Bad
fun process(identifier : String , options : Hash< Symbol , Object >) -> Bool {
  // ...
}
```

## 8. Blank Lines And Layout Spacing

* Top-Level Declarations: Separate top-level class, module, and contract declarations with exactly one blank line.
* Methods: Separate named methods inside a type body with exactly one blank line.
* Grouped Members: Tightly related properties, constants, and import statements may be grouped together with no blank lines between them.
* Block Edges: Do not place blank lines immediately after an opening brace or immediately before a closing brace.
* Attachments: Doc comments (`///`) and decorators (`@decorator(...)`) attach directly to the declaration they document or modify without blank lines between them.
* Blank Runs: Keep at most one consecutive blank line outside protected literal/comment interiors.

```iris
// Good
import Core::IO
import Core::Collections

/// Manages user authentication sessions.
class Session {
  public property id: String
  public property active: Bool

  fun initialize(id: String) -> Nil {
    @id = id
    @active = true
  }

  fun terminate() -> Nil {
    @active = false
  }
}
```

## 9. Comments

1. Marker Spacing: Leave exactly one space after the `//` or `///` comment marker when nonempty text follows. Preserve comment content and block-comment interiors.
2. Trailing Comments: Place exactly two spaces between code and an inline trailing comment on the same line. Do not visually align trailing comments across multiple lines.
3. No Reflow: Do not reflow or re-wrap existing comment blocks during formatting changes.
4. Purpose: Use comments to explain why non-obvious code exists rather than restating what the code does.

```iris
// Good: one space after comment marker
// Compute the decay factor based on elapsed iterations.
let factor = compute_decay()

let limit = 100  // Exactly two spaces before trailing comment

// Bad: missing space after comment marker
//Compute decay
```

## 10. String And Literal Conventions

1. Double Quotes: Use double quotes (`"text"`) as the standard convention for ordinary strings. Single quotes (`'text'`) are valid for strings that must not interpolate.
2. Preserve Existing Content: Never reformat, escape, or alter the characters inside string literals, multiline triple-quoted blocks, raw strings, regex literals, or interpolation blocks (`${...}`).
3. Retain Established Spellings: In native bindings, historical contracts, and API surfaces, preserve existing names and spellings exactly. Never rename identifiers or properties merely to satisfy external conventions.

## 11. Preserving Semantics

Formatting and styling must never alter program semantics. Formatters and engineers must observe these prohibitions:

* Never Reorder Imports: Module imports and bindings must stay in their declared source order. Reordering imports can change module initialization timing and side effects.
* Never Reorder Declarations: Declarations inside modules and classes must remain in their original order.
* Never Reorder Decorators: Multiple decorators evaluate in specific sequence. Keep decorator applications in their declared order.
* Never Alter Types or Return Shapes: Do not strip, rename, or add type annotations or parentheses around callable return types, such as `-> (Integer, String)`. Parenthesized return types carry semantic type meanings.
* No Semantic Assumptions: Style adjustments must remain syntactically transparent and runtime-equivalent.
* No Guessed Repairs: Malformed input, an unsafe transformation, or an uncertain parse must produce no edits rather than a partially repaired program.
* Fixed Official Style: Official formatting uses two spaces and a 120-column soft limit rather than caller-selected layout variants.
