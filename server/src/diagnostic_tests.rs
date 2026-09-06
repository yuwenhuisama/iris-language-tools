use crate::diagnostics::analyze;
use lsp_types::{NumberOrString, Position, Range};

#[test]
fn uses_source_offsets_when_scanner_reports_precise_errors() {
    let cases = [
        ("let good = 1\r\n#! late", "LEX_SHEBANG_NOT_FIRST", (1, 0)),
        ("let good = 1\r/*", "LEX_UNTERMINATED_COMMENT", (1, 0)),
        ("\u{feff}\\", "LEX_BAD_CONTINUATION", (0, 1)),
        (
            "\"\u{1f600}\"; ${x}",
            "LEX_INTERPOLATION_OUTSIDE_LITERAL",
            (0, 6),
        ),
        ("let good = 1; /", "LEX_UNTERMINATED_LITERAL", (0, 14)),
        ("let good = 1; \u{1f600}", "LEX_INVALID_IDENTIFIER", (0, 14)),
        ("let good = 1; rm\"x\"", "LEX_BAD_LITERAL_PREFIX", (0, 14)),
    ];
    for (text, code, (line, column)) in cases {
        let (located, unlocated) = analyze(text);
        assert!(unlocated.is_empty(), "{text:?}: {unlocated:?}");
        assert_eq!(located.len(), 1, "{text:?}");
        assert_eq!(located[0].code, Some(NumberOrString::String(code.into())));
        let position = Position::new(line, column);
        assert_eq!(located[0].range, Range::new(position, position));
    }
}

#[test]
fn omits_squiggles_when_literal_conversion_cannot_locate_errors() {
    let cases = [
        ("1__0", "LEX_BAD_NUMERIC_SEPARATOR"),
        ("0x_FF", "LEX_EMPTY_RADIX_PREFIX"),
        ("0b102", "LEX_INVALID_RADIX_DIGIT"),
        ("1.0F32", "LEX_BAD_FLOAT_SUFFIX"),
        ("\"\\q\"", "LEX_BAD_ESCAPE"),
        ("r###\"x\"##", "LEX_BAD_RAW_FENCE"),
        ("\"\"\"\n  good\n bad\n  \"\"\"", "LEX_BAD_MULTILINE_INDENT"),
    ];
    for (literal, code) in cases {
        let text = format!("\u{feff}let good = 1\r\nlet bad = {literal}");
        let (located, unlocated) = analyze(&text);
        assert!(located.is_empty(), "{text:?}: {located:?}");
        assert_eq!(unlocated, [code]);
    }
}

#[test]
fn stays_lexical_when_program_is_syntactically_invalid_or_effectful() {
    let text = "let = ; print(\"never executed\")";
    let diagnostics = analyze(text);
    assert_eq!(diagnostics, (Vec::new(), Vec::new()));
}
