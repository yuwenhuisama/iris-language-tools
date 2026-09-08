use super::*;
use iris_analysis::{
    DocumentationInfo, ParameterCategory, SignatureInfo, SignatureParameterInfo, Span,
};

fn info() -> SignatureHelpInfo {
    SignatureHelpInfo {
        signature: SignatureInfo {
            label: "read('\u{1f600}', second)".into(),
            parameters: vec![SignatureParameterInfo {
                label: Span { start: 13, end: 19 },
                name: "second".into(),
                category: ParameterCategory::Positional,
                docs: Some(DocumentationInfo {
                    text: "<parameter>".into(),
                    truncated: false,
                }),
            }],
            docs: None,
        },
        active_parameter: Some(0),
    }
}

#[test]
fn renders_parameter_docs_when_offsets_follow_an_astral_scalar() {
    let given = info();

    let when = render(
        given,
        Options {
            format: Format::Markdown,
            label_offsets: true,
        },
    )
    .unwrap();

    let parameters = when.signatures[0].parameters.as_ref().unwrap();
    assert_eq!(parameters[0].label, ParameterLabel::LabelOffsets([11, 17]));
    assert_eq!(
        parameters[0].documentation,
        Some(Documentation::MarkupContent(lsp_types::MarkupContent {
            kind: MarkupKind::Markdown,
            value: "&lt;parameter&gt;".into(),
        }))
    );
}

#[test]
fn suppresses_signature_when_label_range_is_invalid() {
    for span in [
        Span { start: 7, end: 9 },
        Span { start: 19, end: 13 },
        Span {
            start: 13,
            end: 100,
        },
    ] {
        let mut given = info();
        given.signature.parameters[0].label = span;

        let when = render(given, Options::default());

        assert_eq!(when, None);
    }
}

#[test]
fn suppresses_signature_when_active_parameter_is_uncertain_or_out_of_bounds() {
    for active in [None, Some(1), Some(usize::MAX)] {
        let mut given = info();
        given.active_parameter = active;

        let when = render(given, Options::default());

        assert_eq!(when, None);
    }
}
