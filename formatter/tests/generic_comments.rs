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
fn keeps_statement_indent_when_generic_has_trailing_line_comment() {
    golden(
        "let x:Box<Integer> //tail\nlet y=2",
        "let x: Box<Integer>  // tail\nlet y = 2\n",
    );
}

#[test]
fn keeps_body_indent_when_generic_has_trailing_line_comment() {
    golden(
        "fun f(){let x:Box<Integer> //tail\nlet y=2}",
        "fun f() {\n  let x: Box<Integer>  // tail\n  let y = 2\n}\n",
    );
}

#[test]
fn keeps_nested_closers_tight_when_generic_has_trailing_comments() {
    for comment in ["// tail", "/*tail*/"] {
        golden(
            &format!("let x:Box<Array<Integer>> {comment}\nlet y=2"),
            &format!("let x: Box<Array<Integer>>  {comment}\nlet y = 2\n"),
        );
    }
}

#[test]
fn keeps_statement_indent_when_generic_has_trailing_block_comment() {
    golden(
        "let x:Box<Integer> /* keep  all */\nlet y=2",
        "let x: Box<Integer>  /* keep  all */\nlet y = 2\n",
    );
}

#[test]
fn keeps_body_indent_when_generic_has_multiple_trailing_comments() {
    golden(
        "fun f(){let x:Box<Array<Integer>> /*one*/ //tail\n//next\nlet y=2}",
        "fun f() {\n  let x: Box<Array<Integer>>  /*one*/  // tail\n  // next\n  let y = 2\n}\n",
    );
}

#[test]
fn keeps_generic_call_tight_when_block_comments_precede_arguments() {
    golden(
        "let x=identity < Integer > /*one*/ /*two*/ (\" keep  all \" )",
        "let x = identity<Integer>  /*one*/  /*two*/(\" keep  all \")\n",
    );
}

#[test]
fn keeps_generic_member_tight_when_block_comment_precedes_selector() {
    golden(
        "let x=Box < Array<Integer>> /*member*/ .new()",
        "let x = Box<Array<Integer>>  /*member*/.new()\n",
    );
}

#[test]
fn keeps_generic_value_tight_when_line_comment_ends_the_statement() {
    golden(
        "let x=Box<Integer> //tail\nnext()",
        "let x = Box<Integer>  // tail\nnext()\n",
    );
}

#[test]
fn keeps_generic_value_tight_when_block_comment_contains_statement_boundary() {
    for newline in ["\n", "\r\n", "\r"] {
        golden(
            &format!("let x=Box<Integer> /* keep{newline} all */ next()"),
            &format!("let x = Box<Integer>  /* keep{newline} all */ next()\n"),
        );
    }
}

#[test]
fn recognizes_generic_at_end_when_only_comments_follow() {
    for (comment, expected_comment) in [
        ("//tail", "// tail"),
        ("/*tail*/", "/*tail*/"),
        ("/*one*/ //tail", "/*one*/  // tail"),
    ] {
        golden(
            &format!("let x:Box<Integer> {comment}"),
            &format!("let x: Box<Integer>  {expected_comment}\n"),
        );
    }
}

#[test]
fn retains_operator_continuation_when_comparison_has_trailing_comments() {
    for operator in ["<", ">", ">>"] {
        golden(
            &format!("let x=first{operator} /*one*/ //tail\nsecond\nlet y=2"),
            &format!("let x = first {operator}  /*one*/  // tail\n  second\nlet y = 2\n"),
        );
    }
}

#[test]
fn retains_body_continuation_when_comparison_has_trailing_comments() {
    golden(
        "fun f(){let x=first> //tail\nsecond\nlet y=2}",
        "fun f() {\n  let x = first >  // tail\n    second\n  let y = 2\n}\n",
    );
}

#[test]
fn retains_comparison_continuation_when_operand_is_a_commented_generic() {
    golden(
        "let x=first< //operand\nBox<Integer> //tail\nlet y=2",
        "let x = first <  // operand\n  Box<Integer>  // tail\nlet y = 2\n",
    );
}
