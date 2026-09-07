use super::*;
use iris_analysis::Span;
use serde_json::json;

fn info(signature: &str, label: Option<&str>) -> HoverInfo {
    HoverInfo {
        span: Span { start: 0, end: 1 },
        signature: signature.into(),
        type_label: label.map(str::to_owned),
    }
}

#[test]
fn preserves_plaintext_when_signature_contains_markup_and_linebreaks() {
    let given = info("fun read(value = 'a  b\r\n`[go](command:run)` <b>&')", None);

    let when = render(&given, Format::Plaintext);

    assert_eq!(when.value, given.signature);
    assert_eq!(when.kind, MarkupKind::PlainText);
}

#[test]
fn escapes_source_when_markdown_contains_links_html_and_fences() {
    let given = info("[go](command:run) <b>&amp;\n```\n# title", None);

    let when = render(&given, Format::Markdown);

    assert_eq!(
        when.value,
        "\\[go\\]\\(command\\:run\\) \\<b\\>\\&amp\\;\n\\`\\`\\`\n\\# title"
    );
}

#[test]
fn bounds_output_when_escaping_large_backtick_runs() {
    let given = info(&"`".repeat(100_000), None);

    let when = render(&given, Format::Markdown);

    assert!(when.value.len() <= MAX_BYTES);
    assert!(when.value.ends_with("..."));
    assert_eq!(when.value.trim_end_matches('.'), "\\`".repeat(4094));
}

#[test]
fn preserves_scalar_boundaries_when_unicode_exceeds_budget() {
    let given = info(&"\u{1f600}".repeat(3000), None);
    for format in [Format::Plaintext, Format::Markdown] {
        let when = render(&given, format);

        assert!(when.value.len() <= MAX_BYTES);
        assert_eq!(when.value, format!("{}...", "\u{1f600}".repeat(2047)));
    }
}

#[test]
fn displays_type_only_when_not_already_in_signature() {
    for (signature, label, expected) in [
        ("let item: Integer", Some("Integer"), "let item: Integer"),
        (
            "fun read() -> String",
            Some("String"),
            "fun read() -> String",
        ),
        (
            "*values: Integer",
            Some("Array<Integer>"),
            "*values: Integer\n\nType: Array<Integer>",
        ),
        ("class Box", None, "class Box"),
    ] {
        let given = info(signature, label);

        let when = render(&given, Format::Plaintext);

        assert_eq!(when.value, expected);
    }
}

#[test]
fn negotiates_first_supported_format_when_client_lists_preferences() {
    for (formats, expected) in [
        (json!(["markdown", "plaintext"]), Format::Markdown),
        (json!(["plaintext", "markdown"]), Format::Plaintext),
        (json!([]), Format::Plaintext),
    ] {
        let given =
            serde_json::from_value(json!({"textDocument":{"hover":{"contentFormat":formats}}}))
                .unwrap();

        let when = Format::negotiate(&given);

        assert_eq!(when, expected);
    }
    assert_eq!(
        Format::negotiate(&ClientCapabilities::default()),
        Format::Plaintext
    );
}
