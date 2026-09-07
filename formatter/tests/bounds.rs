use iris_formatter::{FormatOutcome, SkipReason, format_document};

#[test]
fn rejects_output_expansion_when_fixed_indentation_exceeds_one_mib() {
    let depth = 8;
    let source = format!(
        "{}{}{}",
        "if ready{".repeat(depth),
        "x\n".repeat(65_000),
        "}".repeat(depth)
    );
    assert!(source.len() < 256 * 1024);
    assert_eq!(
        format_document(&source),
        FormatOutcome::Skipped(SkipReason::OutputLimit)
    );
}

#[test]
fn accepts_normal_depth_when_statements_are_independent() {
    let source = format!("fun f(){{{}}}", "let value=1;".repeat(1000));
    let actual = format_document(&source);
    assert!(matches!(actual, FormatOutcome::Changed(_)), "{actual:?}");
}

#[test]
fn preserves_indivisible_long_literal_when_soft_width_cannot_be_met() {
    let literal = format!("\"{}\"", "x".repeat(121));
    let source = format!("let text={literal}");
    assert_eq!(
        format_document(&source),
        FormatOutcome::Changed(format!("let text = {literal}\n"))
    );
}

#[test]
fn collapses_large_blank_input_when_newline_trivia_reaches_the_input_limit() {
    assert_eq!(
        format_document(&"\n".repeat(256 * 1024)),
        FormatOutcome::Changed(String::new())
    );
}
