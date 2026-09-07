use iris_formatter::{FormatOutcome, format_document};

fn golden(source: &str, expected: &str) {
    let original = iris_parser::parse(source);
    assert!(
        original.program_accepted && original.is_clean(),
        "{source}: {:?}",
        original.diagnostics
    );
    let actual = format_document(source);
    assert_eq!(actual, FormatOutcome::Changed(expected.into()), "{source}");
    assert_eq!(
        format_document(expected),
        FormatOutcome::Unchanged,
        "{expected}"
    );
}

#[test]
fn expands_named_body_when_return_type_starts_on_next_line() {
    golden("fun f() ->\nInteger {1}", "fun f() -> Integer {\n  1\n}\n");
}

#[test]
fn expands_named_body_when_class_clauses_span_lines() {
    golden(
        "class C\nextends Base\n{1}",
        "class C extends Base {\n  1\n}\n",
    );
}

#[test]
fn retains_header_context_when_neighboring_declarations_span_lines() {
    for (source, expected) in [
        (
            "contract C\nextends Base\n{}",
            "contract C extends Base {\n}\n",
        ),
        ("module M\nmixin Base\n{}", "module M mixin Base {\n}\n"),
        (
            "class C\nextends\nBase\nfor Other\n{}",
            "class C extends Base for Other {\n}\n",
        ),
        (
            "fun f() ->\nBox<Integer>\n{1}",
            "fun f() -> Box<Integer> {\n  1\n}\n",
        ),
        (
            "contract C{fun f() ->\nInteger\nfun g(){1}}",
            "contract C {\n  fun f() -> Integer\n\n  fun g() {\n    1\n  }\n}\n",
        ),
        ("fun f()\nlet g={1}", "fun f()\nlet g = { 1 }\n"),
    ] {
        golden(source, expected);
    }
}

#[test]
fn retains_header_context_when_generic_types_span_lines() {
    golden(
        "fun f()->Box<\nInteger\n>{1}",
        "fun f() -> Box<Integer> {\n  1\n}\n",
    );
    golden(
        "contract C\nextends First,\nSecond\n{}",
        "contract C extends First, Second {\n}\n",
    );
}

#[test]
fn indents_type_operand_when_keyword_operator_requires_continuation() {
    for operator in ["is", "as", "as?"] {
        golden(
            &format!("let x=value {operator}\nInteger"),
            &format!("let x = value {operator}\n  Integer\n"),
        );
    }
}

#[test]
fn keeps_mutation_suffix_tight_when_declaring_a_method() {
    golden("fun save!(){1}", "fun save!() {\n  1\n}\n");
}

#[test]
fn keeps_mutation_suffix_tight_when_sending_a_message() {
    golden("obj.save!()", "obj.save!()\n");
}

#[test]
fn distinguishes_suffixes_from_prefixes_when_selectors_have_different_roles() {
    for (source, expected) in [
        (
            "obj.save ! ();obj..save ! ();let name=:save !",
            "obj.save!()\nobj..save!()\nlet name = :save!\n",
        ),
        (
            "fun <(other){1};obj.<(1);obj.!= (1)",
            "fun <(other) {\n  1\n}\nobj.<(1)\nobj.!=(1)\n",
        ),
        (
            "let x=! ready;let y=a < ! ready;let z= ! (ready)",
            "let x = !ready\nlet y = a < !ready\nlet z = !(ready)\n",
        ),
    ] {
        golden(source, expected);
    }
}

#[test]
fn indents_comparison_operand_when_nested_in_a_body() {
    golden(
        "fun f(){let v=a<\nb}",
        "fun f() {\n  let v = a <\n    b\n}\n",
    );
}

#[test]
fn indents_comparison_operand_when_at_top_level() {
    golden("let a=first >\nsecond", "let a = first >\n  second\n");
}

#[test]
fn indents_operands_when_symbolic_operators_require_continuation() {
    for operator in [
        "<", ">", "<=>", "=~", "!~", "**=", "&=", "|=", "^=", "<<=", ">>=", "&&=", "||=",
    ] {
        golden(
            &format!("first {operator}\nsecond"),
            &format!("first {operator}\n  second\n"),
        );
    }
}

#[test]
fn avoids_continuation_indent_when_generic_closers_end_a_statement() {
    golden(
        "let x:Box<Integer>\nlet y:Box<Array<Integer>>\nlet z=1",
        "let x: Box<Integer>\nlet y: Box<Array<Integer>>\nlet z = 1\n",
    );
}

#[test]
fn retains_trailing_comment_when_semicolon_precedes_named_declaration() {
    golden(
        "let x=1; //one\nclass C{}",
        "let x = 1  // one\n\nclass C {\n}\n",
    );
}

#[test]
fn preserves_comment_attachment_when_declaration_has_leading_docs() {
    golden(
        "let x=1; //one\n///doc\n@tag()\nclass C{}",
        "let x = 1  // one\n\n/// doc\n@tag()\nclass C {\n}\n",
    );
    golden(
        "let x=1; /*one*/\nclass C{}",
        "let x = 1  /*one*/\n\nclass C {\n}\n",
    );
}

#[test]
fn retains_standalone_comment_when_it_precedes_an_element_comma() {
    golden(
        "let x=[\n1\n//note\n,2]",
        "let x = [\n  1,\n  // note\n  2,\n]\n",
    );
}

#[test]
fn preserves_comment_lines_when_multiple_comments_follow_an_element() {
    golden(
        "let x=[1 //tail\n//note\n,2]",
        "let x = [\n  1,  // tail\n  // note\n  2,\n]\n",
    );
    golden(
        "let x=[1\n/*note*/\n,2]",
        "let x = [\n  1,\n  /*note*/\n  2,\n]\n",
    );
}

#[test]
fn indents_operand_when_line_comment_follows_operator() {
    golden("let x=a< //note\nb", "let x = a <  // note\n  b\n");
}

#[test]
fn indents_operand_when_operator_comment_is_nested() {
    golden(
        "fun f(){let x=a< //note\nb}",
        "fun f() {\n  let x = a <  // note\n    b\n}\n",
    );
}

#[test]
fn retains_pending_operand_when_multiple_comment_lines_intervene() {
    golden(
        "let x=a< //note\n//more\n/* keep  all */\n//last\nb\nlet y=2",
        "let x = a <  // note\n  // more\n  /* keep  all */\n  // last\n  b\nlet y = 2\n",
    );
}

#[test]
fn retains_pending_operand_when_block_comments_follow_a_break() {
    for (source, expected) in [
        ("let x=a<\n/*note*/\nb", "let x = a <\n  /*note*/\n  b\n"),
        (
            "fun f(){let x=a< /*one*/\n/* keep\r\n  all */\nb;let y=2}",
            "fun f() {\n  let x = a <  /*one*/\n    /* keep\r\n  all */\n    b\n  let y = 2\n}\n",
        ),
        (
            "let x=a< //note\n\" keep  all \" //tail\n//next\nlet y=2",
            "let x = a <  // note\n  \" keep  all \"  // tail\n// next\nlet y = 2\n",
        ),
    ] {
        golden(source, expected);
    }
}

#[test]
fn places_comma_after_element_when_leading_and_standalone_comments_coexist() {
    golden(
        "let x=[\n//leading\n1\n//note\n,2]",
        "let x = [\n  // leading\n  1,\n  // note\n  2,\n]\n",
    );
}

#[test]
fn places_comma_before_tail_when_element_also_has_leading_comment() {
    golden(
        "let x=[\n//leading\n1 //tail\n,2]",
        "let x = [\n  // leading\n  1,  // tail\n  2,\n]\n",
    );
}

#[test]
fn classifies_comments_when_neighboring_elements_have_mixed_attachments() {
    golden(
        "let x=[\n//leading\n/*before*/\n1 //tail\n//note\n,2 /*after*/\n//last\n]",
        "let x = [\n  // leading\n  /*before*/\n  1,  // tail\n  // note\n  2,  /*after*/\n  // last\n]\n",
    );
}

#[test]
fn retains_internal_block_comments_when_placing_element_comma() {
    for (source, expected) in [
        (
            "let x=[\n//leading\n1/*inner*/+2 /*tail*/\n//note\n,3]",
            "let x = [\n  // leading\n  1  /*inner*/ + 2,  /*tail*/\n  // note\n  3,\n]\n",
        ),
        (
            "let x=[/*leading*/1/*inner*/+2/*tail*/,3]",
            "let x = [\n  /*leading*/1  /*inner*/ + 2,  /*tail*/\n  3,\n]\n",
        ),
    ] {
        golden(source, expected);
    }
}
