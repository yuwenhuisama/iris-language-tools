use iris_formatter::{FormatOutcome, SkipReason, format_document};

#[test]
fn preserves_literal_bytes_when_every_supported_literal_family_occurs() {
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
        "\"\"\"\r\n  body\r\n  \"\"\"",
        "/[{}()]+/im",
        "\"} [ ( // /*\"",
        "b\"hi\"",
        "m\"text\"",
        "mb\"data\"",
        "'\"'",
        "\"\\\"}\"",
        "0x1.fp3f32",
    ] {
        let source = format!("fun f(){{let text={literal};}}");
        let expected = format!("fun f() {{\n  let text = {literal}\n}}\n");
        assert_eq!(
            format_document(&source),
            FormatOutcome::Changed(expected.clone()),
            "{literal}"
        );
        assert_eq!(
            format_document(&expected),
            FormatOutcome::Unchanged,
            "{literal}"
        );
    }
}

#[test]
fn skips_continuations_when_newline_is_escaped_outside_literals() {
    for newline in ["\n", "\r\n", "\r"] {
        let source = format!("fun f() {{\n1 + \\{newline}2\n}}");
        assert_eq!(
            format_document(&source),
            FormatOutcome::Skipped(SkipReason::Continuation)
        );
    }
}

#[test]
fn skips_source_when_lexer_reports_diagnostics() {
    for source in [
        "fun f(){1__0}",
        "/* open",
        "\"unclosed",
        "let bad = \\",
        "\"\\q\"",
        "\u{200b}",
    ] {
        assert_eq!(
            format_document(source),
            FormatOutcome::Skipped(SkipReason::LexicalDiagnostics),
            "{source}"
        );
    }
}

#[test]
fn skips_source_when_delimiters_do_not_match() {
    for source in ["{\n)", "[\n}", "%{\n]", "(\n]", "}", "([)]"] {
        assert_eq!(
            format_document(source),
            FormatOutcome::Skipped(SkipReason::MismatchedDelimiter)
        );
    }
}

#[test]
fn skips_source_when_delimiters_remain_open() {
    for source in ["{\nprint(1)", "[", "%{", "("] {
        assert_eq!(
            format_document(source),
            FormatOutcome::Skipped(SkipReason::UnclosedDelimiter)
        );
    }
}

#[test]
fn skips_input_when_byte_budget_is_exceeded() {
    assert_eq!(
        format_document(&" ".repeat(256 * 1024 + 1)),
        FormatOutcome::Skipped(SkipReason::InputLimit)
    );
}

#[test]
fn accepts_input_when_byte_budget_is_exact() {
    assert_eq!(
        format_document(&" ".repeat(256 * 1024)),
        FormatOutcome::Changed(String::new())
    );
}

#[test]
fn skips_nesting_when_stack_budget_is_exceeded() {
    let source = format!("{}{}", "(".repeat(129), ")".repeat(129));
    assert_eq!(
        format_document(&source),
        FormatOutcome::Skipped(SkipReason::NestingLimit)
    );
}

#[test]
fn declines_parser_resource_limit_when_delimiter_budget_is_exact() {
    let source = format!("{}x{}", "(".repeat(128), ")".repeat(128));
    assert_eq!(
        format_document(&source),
        FormatOutcome::Skipped(SkipReason::ParseDiagnostics)
    );
}

#[test]
fn rejects_malformed_syntax_when_tokens_are_lexically_clean() {
    for source in [
        ";print(1)",
        "let = 1",
        "fun f(,){}",
        "let f={|x| x}",
        "if ready {} else",
        "x;;y",
    ] {
        assert_eq!(
            format_document(source),
            FormatOutcome::Skipped(SkipReason::ParseDiagnostics),
            "{source}"
        );
    }
}

#[test]
fn preserves_newline_meaning_when_calls_symbols_and_arrays_are_adjacent() {
    for source in [
        "fun f(){return\n1}",
        "let a = [1]\n[2]",
        "f(x: :a)",
        "obj.+(1)",
        "property fun ready?=(v: Bool) {}",
    ] {
        let outcome = format_document(source);
        assert!(
            !matches!(outcome, FormatOutcome::Skipped(_)),
            "{source}: {outcome:?}"
        );
        if let FormatOutcome::Changed(output) = outcome {
            assert_eq!(format_document(&output), FormatOutcome::Unchanged);
        }
    }
}
