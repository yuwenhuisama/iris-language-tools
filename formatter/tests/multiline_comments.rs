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
fn keeps_unary_tight_when_comment_ends_top_level_statement() {
    for operand in [
        "Box<Integer>",
        "Box<Array<Integer>>",
        "value",
        "previous()",
        "1",
    ] {
        for operator in ["+", "-", "!", "~"] {
            for newline in ["\n", "\r\n", "\r"] {
                golden(
                    &format!("let x={operand} /*tail{newline}keep*/ {operator} next()\nlet y=2"),
                    &format!(
                        "let x = {operand}  /*tail{newline}keep*/ {operator}next()\nlet y = 2\n"
                    ),
                );
            }
        }
    }
}

#[test]
fn keeps_unary_and_indent_when_comment_ends_nested_statement() {
    for operand in ["Box<Integer>", "value", "previous()"] {
        for operator in ["+", "-"] {
            for newline in ["\n", "\r\n"] {
                golden(
                    &format!(
                        "fun f(){{let x={operand} /*tail{newline}keep*/ {operator} next()\nlet y=2}}"
                    ),
                    &format!(
                        "fun f() {{\n  let x = {operand}  /*tail{newline}keep*/ {operator}next()\n  let y = 2\n}}\n"
                    ),
                );
            }
        }
    }
}

#[test]
fn retains_binary_spacing_when_comment_newline_is_delimited() {
    for operand in ["Box<Integer>", "value", "previous()"] {
        for operator in ["+", "-"] {
            for (open, close, comma) in [
                ("[", "]", ","),
                ("call(", ")", ","),
                ("(", ")", ""),
                ("values[", "]", ""),
                ("%{:key:", "}", ","),
            ] {
                let entry = if open == "%{:key:" { "%{" } else { open };
                let key = if open == "%{:key:" { ":key: " } else { "" };
                golden(
                    &format!("let x={open}{operand} /*tail\r\nkeep*/ {operator}next(){close}"),
                    &format!(
                        "let x = {entry}\n  {key}{operand}  /*tail\r\nkeep*/ {operator} next(){comma}\n{close}\n"
                    ),
                );
            }
        }
    }
}

#[test]
fn retains_pending_operand_when_operator_precedes_comment() {
    for operator in ["+", "-", "<", ">", "=", "*"] {
        for unary in ["+", "-"] {
            golden(
                &format!(
                    "fun f(){{let x=value {operator} /*tail\r\nkeep*/ {unary} next()\nlet y=2}}"
                ),
                &format!(
                    "fun f() {{\n  let x = value {operator}  /*tail\r\nkeep*/ {unary}next()\n  let y = 2\n}}\n"
                ),
            );
        }
    }
}

#[test]
fn restores_base_indent_when_continued_operand_ends_at_comment() {
    golden(
        "fun f(){let x=\nBox<Integer> /*tail\nkeep*/ -next() {1}\nlet y=2}",
        "fun f() {\n  let x =\n    Box<Integer>  /*tail\nkeep*/ -next() { 1 }\n  let y = 2\n}\n",
    );
}

#[test]
fn keeps_unary_tight_when_statement_body_is_inside_a_list() {
    golden(
        "call({let x=Box<Integer> /*tail\nkeep*/ -next()\nlet y=2})",
        "call(\n  {\n    let x = Box<Integer>  /*tail\nkeep*/ -next()\n    let y = 2\n  },\n)\n",
    );
}

#[test]
fn retains_inline_binary_spacing_when_comment_has_no_newline() {
    golden(
        "let x=value /*tail*/ -next()",
        "let x = value  /*tail*/ - next()\n",
    );
}

#[test]
fn retains_binary_spacing_when_delimited_comment_has_exterior_newlines() {
    for operand in ["Box<Integer>", "value", "previous()"] {
        for operator in ["+", "-"] {
            for (before, after) in [("", "\n"), ("\n", ""), ("\n", "\n")] {
                golden(
                    &format!("let x=[{operand}{before} /*tail\nkeep*/{after}{operator}next()]"),
                    &format!(
                        "let x = [\n  {operand}{}/*tail\nkeep*/{}{operator} next(),\n]\n",
                        if before.is_empty() { "  " } else { "\n  " },
                        if after.is_empty() { " " } else { "\n  " },
                    ),
                );
            }
        }
    }
}

#[test]
fn retains_header_and_trailing_block_indent_when_comments_contain_newlines() {
    for (source, expected) in [
        (
            "fun f() /*tail\nkeep*/ {1}\nfun g(){2}",
            "fun f()  /*tail\nkeep*/ {\n  1\n}\n\nfun g() {\n  2\n}\n",
        ),
        (
            "fun f()-> /*tail\nkeep*/ Box<Integer> {1}",
            "fun f() ->  /*tail\nkeep*/ Box<Integer> {\n  1\n}\n",
        ),
        (
            "fun f(){let x=\nvalue /*tail\nkeep*/ -next() {let z=1\nz}\nlet y=2}",
            "fun f() {\n  let x =\n    value  /*tail\nkeep*/ -next() {\n    let z = 1\n    z\n  }\n  let y = 2\n}\n",
        ),
    ] {
        golden(source, expected);
    }
}

#[test]
fn retains_operand_roles_when_comments_surround_delimited_operators() {
    for operator in ["+", "-"] {
        for (comment, expected) in [
            ("/*tail\nkeep*/", "/*tail\nkeep*/ "),
            ("//tail\n", "// tail\n  "),
        ] {
            golden(
                &format!("let x=[value {operator} {comment} - next()]"),
                &format!("let x = [\n  value {operator}  {expected}-next(),\n]\n"),
            );
        }
    }
}
