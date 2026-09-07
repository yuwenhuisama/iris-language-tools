use iris_formatter::{FormatOptions, FormatOutcome, InvalidTabSize, SkipReason, format_document};

#[test]
fn rejects_options_when_tab_size_is_outside_bounds() {
    for tab_size in [0, 17, u32::MAX] {
        let actual = FormatOptions::new(tab_size, true);

        assert_eq!(actual, Err(InvalidTabSize(tab_size)));
    }
}

#[test]
fn skips_literals_when_protected_spans_are_ambiguous() {
    for literal in [
        "r\"raw\"",
        "r#\"raw\"#",
        "br##\"raw\"##",
        "mr'raw'",
        "mbr\"raw\"",
        "\"\"\"triple\"\"\"",
        "'''triple'''",
        "\"${value}\"",
        "m\"${value}\"",
        "\"\"\"\n  body\n  \"\"\"",
    ] {
        let source = format!("fun f() {{\nlet text = {literal}\n}}");

        let actual = format_document(&source, FormatOptions::new(2, true).unwrap());

        assert_eq!(
            actual,
            FormatOutcome::Skipped(SkipReason::UnsupportedLiteral),
            "{literal}"
        );
    }
}

#[test]
fn skips_continuations_when_newline_is_escaped() {
    for newline in ["\n", "\r\n", "\r"] {
        let source = format!("fun f() {{\n1 + \\{newline}2\n}}");

        let actual = format_document(&source, FormatOptions::new(2, true).unwrap());

        assert_eq!(actual, FormatOutcome::Skipped(SkipReason::Continuation));
    }
}

#[test]
fn skips_source_when_lexer_reports_diagnostics() {
    for source in [
        "fun f() {\n1__0\n}",
        "/* open",
        "\"unclosed",
        "let bad = \\",
        "\"\\q\"",
        "\u{200b}",
        "fun f() {\nprint(1+2)\n}",
    ] {
        let actual = format_document(source, FormatOptions::new(2, true).unwrap());

        assert_eq!(
            actual,
            FormatOutcome::Skipped(SkipReason::LexicalDiagnostics),
            "{source}"
        );
    }
}

#[test]
fn skips_source_when_delimiters_do_not_match() {
    for source in ["{\n)", "[\n}", "%{\n]", "(\n]", "}", "([)]"] {
        let actual = format_document(source, FormatOptions::new(2, true).unwrap());

        assert_eq!(
            actual,
            FormatOutcome::Skipped(SkipReason::MismatchedDelimiter)
        );
    }
}

#[test]
fn skips_source_when_delimiters_remain_open() {
    for source in ["{\nprint(1)", "[", "%{", "("] {
        let actual = format_document(source, FormatOptions::new(2, true).unwrap());

        assert_eq!(
            actual,
            FormatOutcome::Skipped(SkipReason::UnclosedDelimiter)
        );
    }
}

#[test]
fn skips_input_when_byte_budget_is_exceeded() {
    let source = " ".repeat(256 * 1024 + 1);

    let actual = format_document(&source, FormatOptions::new(2, true).unwrap());

    assert_eq!(actual, FormatOutcome::Skipped(SkipReason::InputLimit));
}

#[test]
fn accepts_input_when_byte_budget_is_exact() {
    let source = " ".repeat(256 * 1024);

    let actual = format_document(&source, FormatOptions::new(2, true).unwrap());

    assert_eq!(actual, FormatOutcome::Unchanged);
}

#[test]
fn skips_nesting_when_stack_budget_is_exceeded() {
    let source = format!("{}{}", "(".repeat(129), ")".repeat(129));

    let actual = format_document(&source, FormatOptions::new(2, true).unwrap());

    assert_eq!(actual, FormatOutcome::Skipped(SkipReason::NestingLimit));
}

#[test]
fn accepts_nesting_when_stack_budget_is_exact() {
    let source = format!("{}\nx\n{}", "(".repeat(128), ")".repeat(128));

    let actual = format_document(&source, FormatOptions::new(1, false).unwrap());

    assert_eq!(
        actual,
        FormatOutcome::Changed(format!(
            "{}\n{}x\n{}",
            "(".repeat(128),
            "\t".repeat(128),
            ")".repeat(128)
        ))
    );
}

#[test]
fn skips_output_when_indentation_expansion_exceeds_budget() {
    let source = format!(
        "{}\n{}{}",
        "{".repeat(128),
        "x\n".repeat(600),
        "}".repeat(128)
    );

    let actual = format_document(&source, FormatOptions::new(16, true).unwrap());

    assert_eq!(actual, FormatOutcome::Skipped(SkipReason::OutputLimit));
}
