use iris_formatter::{FormatOutcome, format_document};

fn golden(source: &str, expected: &str) {
    assert!(
        iris_parser::parse(source).program_accepted,
        "{source}: {:?}",
        iris_parser::parse(source).diagnostics
    );
    assert_eq!(
        format_document(source),
        FormatOutcome::Changed(expected.into()),
        "{source}"
    );
    assert_eq!(
        format_document(expected),
        FormatOutcome::Unchanged,
        "{expected}"
    );
}

#[test]
fn preserves_parameter_channels_when_prefixes_must_be_tight() {
    golden(
        "fun f(first:Integer,* rest,key second:String=\"x\",** options,& body:Block<()->Integer>){first}",
        "fun f(first: Integer, *rest, key second: String = \"x\", **options, &body: Block<() -> Integer>) {\n  first\n}\n",
    );
}

#[test]
fn keeps_bodyless_requirements_separate_when_contract_methods_have_no_braces() {
    golden(
        "contract C{fun a()->Integer\nfun b()->String}",
        "contract C {\n  fun a() -> Integer\n\n  fun b() -> String\n}\n",
    );
}

#[test]
fn wraps_parameter_lists_when_each_parameter_has_a_type() {
    golden(
        "fun f(\na:Integer,b:String\n){a}",
        "fun f(\n  a: Integer,\n  b: String,\n) {\n  a\n}\n",
    );
}

#[test]
fn collapses_closure_header_spacing_when_bars_and_arrow_are_separate() {
    golden(
        "let f={ | x : Integer | -> Integer\nx+1\n}",
        "let f = { |x: Integer| -> Integer; x + 1 }\n",
    );
}

#[test]
fn formats_nested_delimiters_when_multiple_closers_share_a_line() {
    golden(
        "fun f(){let data=%{\nkey:[\ncall(\n1\n)]\n}}",
        "fun f() {\n  let data = %{\n    key: [\n      call(\n        1,\n      ),\n    ],\n  }\n}\n",
    );
}

#[test]
fn keeps_generic_method_calls_tight_when_types_are_explicit() {
    golden(
        "let a=identity < Integer > (1);let b=Box < String > .new();type Pair<T,U> =Map<T,U>",
        "let a = identity<Integer>(1)\nlet b = Box<String>.new()\n\ntype Pair<T, U> = Map<T, U>\n",
    );
}

#[test]
fn leaves_import_spec_order_unchanged_when_spacing_is_normalized() {
    golden(
        "from Z import b as second,a as first\nimport A::B as C",
        "from Z import b as second, a as first\nimport A::B as C\n",
    );
}

#[test]
fn keeps_generic_commas_inside_a_single_parameter_when_lists_wrap() {
    golden(
        "fun f(\nvalue:Map<String,Integer>,other:Integer\n){value}",
        "fun f(\n  value: Map<String, Integer>,\n  other: Integer,\n) {\n  value\n}\n",
    );
}

#[test]
fn retains_index_role_when_receiver_is_a_call() {
    golden("foo()[\n0\n]", "foo()[\n  0\n]\n");
}

#[test]
fn preserves_separate_expression_statements_when_closure_body_cannot_be_inline() {
    golden("let f={a;b;c}", "let f = {\n  a\n  b\n  c\n}\n");
}

#[test]
fn indents_continued_expressions_when_operator_requires_an_operand() {
    golden("let sum=first+\nsecond", "let sum = first +\n  second\n");
}

#[test]
fn wraps_binary_expressions_when_tokens_can_break_at_an_operator() {
    let left = "a".repeat(70);
    let right = "b".repeat(60);
    golden(
        &format!("let sum={left}+{right}"),
        &format!("let sum = {left} +\n  {right}\n"),
    );
}
