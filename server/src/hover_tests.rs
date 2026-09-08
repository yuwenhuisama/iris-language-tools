use super::*;
use iris_analysis::{DocumentationInfo, HoverDetail, HoverKind, Span};
use serde_json::json;

fn info(signature: &str, label: Option<&str>) -> HoverInfo {
    HoverInfo {
        span: Span { start: 0, end: 1 },
        signature: signature.into(),
        type_label: label.map(str::to_owned),
        kind: HoverKind::Method,
        owner: None,
        details: vec![],
        docs: None,
    }
}

#[test]
fn preserves_plaintext_when_signature_contains_markup_and_linebreaks() {
    let given = info("fun read(value = 'a  b\r\n`[go](command:run)` <b>&')", None);

    let when = render(&given, Format::Plaintext);

    assert_eq!(when.value, format!("{}\n\nKind: Method", given.signature));
    assert_eq!(when.kind, MarkupKind::PlainText);
}

#[test]
fn fences_source_when_markdown_contains_links_html_and_fences() {
    let given = info("[go](command:run) <b>&amp;\n```\n# title", None);

    let when = render(&given, Format::Markdown);

    assert_eq!(
        when.value,
        "````iris\n[go](command:run) <b>&amp;\n```\n# title\n````\n\n**Kind:** Method"
    );
}

#[test]
fn bounds_output_when_fencing_large_backtick_runs() {
    let given = info(&"`".repeat(100_000), None);

    let when = render(&given, Format::Markdown);

    assert!(when.value.len() <= MAX_BYTES);
    let lines: Vec<_> = when.value.lines().collect();
    let fence = lines[0].strip_suffix("iris").unwrap();
    assert_eq!(lines[2], fence);
    assert!(lines[1].ends_with("..."));
    assert!(fence.len() > lines[1].trim_end_matches('.').len());
}

#[test]
fn preserves_scalar_boundaries_when_unicode_exceeds_budget() {
    let given = info(&"\u{1f600}".repeat(3000), None);
    for format in [Format::Plaintext, Format::Markdown] {
        let when = render(&given, format);

        assert!(when.value.len() <= MAX_BYTES);
        assert!(when.value.contains("..."));
        assert!(when.value.contains('\u{1f600}'));
    }
}

#[test]
fn displays_kind_when_signature_already_contains_type() {
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

        assert_eq!(when.value, format!("{expected}\n\nKind: Method"));
    }
}

#[test]
fn separates_metadata_and_literal_docs_when_source_has_rich_details() {
    let mut given = info("public fun read() -> String", Some("String"));
    given.owner = Some("Core::Box".into());
    given.details = vec![HoverDetail::ReturnType("String".into())];
    given.docs = Some(DocumentationInfo {
        text: "Read safely.\n\n![image](https://example.test/a) <b> [run](command:run)\n@param stays plain".into(),
        truncated: false,
    });

    let when = render(&given, Format::Markdown);

    assert!(
        when.value
            .starts_with("```iris\npublic fun read() -> String\n```")
    );
    assert!(when.value.contains("**Owner:** Core\\:\\:Box"));
    assert!(when.value.contains("**Declared return:** String"));
    assert!(
        when.value
            .contains("\n\n---\n\n**Documentation**\n\nRead safely\\.\n\n\\!\\[image\\]")
    );
    assert!(!when.value.contains("<b>"));
    assert!(!when.value.contains("[run](command:run)"));
    assert!(when.value.contains("\\@param stays plain"));
}

#[test]
fn keeps_plaintext_docs_when_plaintext_is_negotiated() {
    let given = DocumentationInfo {
        text: "<b> [run](command:run)\n\n@param plain".into(),
        truncated: false,
    };

    let when = render_docs(&given, Format::Plaintext);

    assert_eq!(when.value, given.text);
    assert_eq!(when.kind, MarkupKind::PlainText);
}

#[test]
fn escapes_complete_scalars_when_documentation_exceeds_final_budget() {
    let given = DocumentationInfo {
        text: "😀[x](command:run)<img src='https://example.test'>".repeat(4000),
        truncated: false,
    };

    let when = render_docs(&given, Format::Markdown);

    assert!(when.value.len() <= MAX_BYTES);
    assert!(when.value.ends_with("..."));
    assert!(!when.value.ends_with("\\..."));
    assert!(!when.value.contains("<img"));
    assert!(!when.value.contains("[x]("));
}

#[test]
fn marks_analysis_truncation_when_documentation_was_already_bounded() {
    let given = DocumentationInfo {
        text: "Partial text".into(),
        truncated: true,
    };

    let when = render_docs(&given, Format::Markdown);

    assert_eq!(when.value, "Partial text...");
}

#[test]
fn reserves_balanced_sections_when_every_field_is_large() {
    let mut given = info(&"😀`".repeat(9000), None);
    given.owner = Some("[owner](command:run)".repeat(9000));
    given.docs = Some(DocumentationInfo {
        text: "<img>".repeat(9000),
        truncated: false,
    });

    let when = render(&given, Format::Markdown);

    assert!(when.value.len() <= MAX_BYTES);
    let fence = when
        .value
        .lines()
        .next()
        .unwrap()
        .strip_suffix("iris")
        .unwrap();
    assert!(when.value.contains(&format!("\n{fence}\n")));
    assert_eq!(when.value.matches("**").count() % 2, 0);
    assert!(!when.value.contains("<img>"));
}

#[test]
fn keeps_entities_and_escapes_atomic_when_budget_ends_at_each_boundary() {
    for padding in 0..24 {
        let given = DocumentationInfo {
            text: format!("{}<>&\\[😀", "x".repeat(MAX_BYTES - padding)),
            truncated: false,
        };

        let when = render_docs(&given, Format::Markdown);

        assert!(when.value.len() <= MAX_BYTES);
        let suffix = when.value.trim_end_matches('.');
        assert!(!suffix.ends_with('&'));
        assert!(!suffix.ends_with("&l"));
        assert!(!suffix.ends_with("&lt"));
        assert!(!suffix.ends_with("&g"));
        assert!(!suffix.ends_with("&gt"));
        assert!(!suffix.ends_with("&a"));
        assert!(!suffix.ends_with("&am"));
        assert!(!suffix.ends_with("&amp"));
        assert_eq!(
            suffix
                .chars()
                .rev()
                .take_while(|scalar| *scalar == '\\')
                .count()
                % 2,
            0
        );
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
