use iris_formatter::{FormatOutcome, format_document};

fn golden(source: &str, expected: &str) {
    let original = iris_parser::parse(source);
    assert!(
        original.program_accepted,
        "{source}: {:?}",
        original.diagnostics
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
fn spaces_symbolic_calls_when_selectors_are_operators() {
    golden(
        "obj . + (1);let x=: +;let y=:foo",
        "obj.+(1)\nlet x = :+\nlet y = :foo\n",
    );
}

#[test]
fn keeps_comments_with_list_items_when_trailing_commas_are_inserted() {
    golden(
        "let a=[\n1,//one\n2 //two\n]",
        "let a = [\n  1,  // one\n  2,  // two\n]\n",
    );
}

#[test]
fn retains_nested_closures_when_an_earlier_property_exists() {
    golden(
        "class A{property value:Integer\nlet f={|x|;x+1}\nfun run(){let g={||;2};g()}}",
        "class A {\n  property value: Integer\n  let f = { |x|; x + 1 }\n\n  fun run() {\n    let g = { ||; 2 }\n    g()\n  }\n}\n",
    );
}

#[test]
fn formats_match_arms_when_arrows_are_compact() {
    golden(
        "match status{1=>handle();2=>wait();else=>stop()}",
        "match status {\n  1 => handle()\n  2 => wait()\n  else => stop()\n}\n",
    );
}

#[test]
fn collapses_excess_blank_lines_when_properties_and_methods_are_grouped() {
    golden(
        "class A{\n\nproperty a:Integer\n\n\nproperty b:Integer\n\nfun f(){1}\n\n\nfun g(){2}\n\n}",
        "class A {\n  property a: Integer\n  property b: Integer\n\n  fun f() {\n    1\n  }\n\n  fun g() {\n    2\n  }\n}\n",
    );
}

#[test]
fn preserves_headerless_simple_closures_when_source_uses_trailing_blocks() {
    golden(
        "call(){1};let f={1};let g={||\n1}",
        "call() { 1 }\nlet f = { 1 }\nlet g = { ||; 1 }\n",
    );
}

#[test]
fn retains_newline_terminated_return_when_closure_is_not_an_expression() {
    golden("let f={||;return;1}", "let f = { ||;\n  return\n  1\n}\n");
}

#[test]
fn keeps_literal_contents_when_marker_text_would_trigger_old_blanket_skips() {
    golden(
        "fun f(){let text=\"r\\\" ${x} \\\\n\";// r\"\"\"\n}",
        "fun f() {\n  let text = \"r\\\" ${x} \\\\n\"  // r\"\"\"\n}\n",
    );
}
