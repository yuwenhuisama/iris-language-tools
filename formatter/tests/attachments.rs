use iris_formatter::{FormatOutcome, format_document};

fn golden(source: &str, expected: &str) {
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
fn keeps_comments_on_their_line_when_they_follow_a_closing_body() {
    golden(
        "fun first(){1}//one\n// lead\nfun second(){2}",
        "fun first() {\n  1\n}  // one\n\n// lead\nfun second() {\n  2\n}\n",
    );
}

#[test]
fn preserves_comments_inside_lists_when_they_precede_the_first_item() {
    golden(
        "let x=[\n//one\n1,\n//two\n2\n]",
        "let x = [\n  // one\n  1,\n  // two\n  2,\n]\n",
    );
}

#[test]
fn preserves_comments_after_the_final_comma_when_a_list_closes() {
    golden("let x=[1,\n// end\n]", "let x = [\n  1,\n  // end\n]\n");
}

#[test]
fn avoids_reflow_when_block_comments_are_inline() {
    golden(
        "let x=1/* keep  all */+2",
        "let x = 1  /* keep  all */ + 2\n",
    );
}

#[test]
fn preserves_required_accessor_semicolons_when_comments_force_multiline() {
    golden(
        "property value:Integer{get;//read\nprivate set;}",
        "property value: Integer {\n  get;  // read\n  private set;\n}\n",
    );
}
