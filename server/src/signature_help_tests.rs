use super::*;
use iris_analysis::{
    DocumentationInfo, ParameterCategory, SignatureInfo, SignatureParameterInfo, Span,
};

fn info() -> SignatureHelpInfo {
    SignatureHelpInfo {
        signatures: vec![SignatureInfo {
            active_parameter: Some(0),
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
        }],
        active_signature: 0,
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
        given.signatures[0].parameters[0].label = span;

        let when = render(given, Options::default());

        assert_eq!(when, None);
    }
}

#[test]
fn suppresses_signature_when_active_parameter_is_uncertain_or_out_of_bounds() {
    for active in [None, Some(1), Some(usize::MAX)] {
        let mut given = info();
        given.signatures[0].active_parameter = active;

        let when = render(given, Options::default());

        assert_eq!(when, None);
    }
}

#[test]
fn renders_independent_parameters_when_multiple_candidates_have_different_shapes() {
    let mut given = info();
    let mut second = given.signatures[0].clone();
    second.label = "read()".into();
    second.parameters.clear();
    second.active_parameter = None;
    given.signatures.push(second);
    given.active_signature = 1;

    let when = render(given, Options::default()).unwrap();

    assert_eq!(when.active_signature, Some(1));
    assert_eq!(when.active_parameter, None);
    assert_eq!(when.signatures[0].active_parameter, Some(0));
    assert_eq!(when.signatures[1].active_parameter, None);
}

#[test]
fn converts_utf16_ranges_when_every_candidate_has_distinct_unicode_labels() {
    let mut given = info();
    let mut second = given.signatures[0].clone();
    second.label = "read('\u{1f600}\u{1f600}', second)".into();
    second.parameters[0].label = Span { start: 17, end: 23 };
    given.signatures.push(second);

    let when = render(
        given,
        Options {
            label_offsets: true,
            ..Options::default()
        },
    )
    .unwrap();

    for (signature, expected) in when.signatures.iter().zip([[11, 17], [13, 19]]) {
        assert_eq!(
            signature.parameters.as_ref().unwrap()[0].label,
            ParameterLabel::LabelOffsets(expected)
        );
        assert_eq!(signature.active_parameter, Some(0));
    }
}

#[test]
fn suppresses_help_when_unselected_candidate_would_inherit_false_highlight() {
    let mut given = info();
    let mut second = given.signatures[0].clone();
    second.active_parameter = None;
    given.signatures.push(second);

    let when = render(given, Options::default());

    assert_eq!(when, None);
}

#[test]
fn renders_filtered_shapes_when_real_analysis_maps_keywords() {
    let text = "module Main { JSON.decode(value, depth: 2) }";
    let given = iris_analysis::AnalysisSnapshot::new([iris_analysis::SourceInput {
        id: iris_analysis::FileId(1),
        group: iris_analysis::GroupId(1),
        text: text.into(),
    }]);

    let when = render(
        given
            .signature_help(iris_analysis::FileId(1), text.find("2)").unwrap())
            .unwrap(),
        Options {
            label_offsets: true,
            ..Options::default()
        },
    )
    .unwrap();

    assert_eq!(when.signatures.len(), 1);
    assert_eq!(when.active_parameter, Some(1));
    assert_eq!(
        when.signatures[0].parameters.as_ref().unwrap()[1].label,
        ParameterLabel::LabelOffsets([25, 43])
    );
}

#[test]
fn suppresses_help_when_real_trailing_block_leaves_no_parenthesized_slot() {
    let text = "module Main { using(resource,) { ||; 1 } }";
    let given = iris_analysis::AnalysisSnapshot::new([iris_analysis::SourceInput {
        id: iris_analysis::FileId(1),
        group: iris_analysis::GroupId(1),
        text: text.into(),
    }]);

    let when = render(
        given
            .signature_help(iris_analysis::FileId(1), text.find(",)").unwrap() + 1)
            .unwrap(),
        Options::default(),
    );

    assert_eq!(when, None);
}
