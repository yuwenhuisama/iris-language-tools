use iris_analysis::{
    AnalysisSnapshot, FileId, GroupId, ParameterCategory, SignatureHelpInfo, SourceInput,
};
use iris_parser::source::ArgumentKind;

fn help(marked: &str) -> Option<SignatureHelpInfo> {
    let byte = marked.find('^').unwrap();
    let text = marked.replace('^', "");
    let given = AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }]);

    given.signature_help(FileId(1), byte)
}

#[test]
fn retains_only_compatible_shapes_when_arguments_identify_channels() {
    for (call, count, active, category) in [
        ("(1 ..= 3).by(step: ^2)", 1, 0, ParameterCategory::Keyword),
        ("(1 ..= 3).by(^2)", 1, 0, ParameterCategory::Positional),
        (
            "[1].reduce(0, ^callback)",
            2,
            1,
            ParameterCategory::Positional,
        ),
        ("[1].reduce(^callback)", 1, 0, ParameterCategory::Positional),
        ("[1].count(^callback)", 1, 0, ParameterCategory::Positional),
        (
            "JSON.encode(value, canonical: ^true)",
            2,
            1,
            ParameterCategory::Keyword,
        ),
        (
            "JSON.decode(value, depth: ^2)",
            2,
            1,
            ParameterCategory::Keyword,
        ),
        (
            "using(resource, ^callback)",
            2,
            1,
            ParameterCategory::Positional,
        ),
    ] {
        let given = format!("module Main {{ {call} }}");

        let when = help(&given).unwrap();

        assert_eq!(when.signatures.len(), 1, "{call}: {when:?}");
        let signature = &when.signatures[when.active_signature];
        assert_eq!(signature.parameters.len(), count, "{call}");
        assert_eq!(signature.active_parameter, Some(active), "{call}");
        assert_eq!(signature.parameters[active].category, category, "{call}");
    }
}

#[test]
fn rejects_shapes_when_any_argument_is_incompatible_even_after_cursor() {
    for call in [
        "JSON.encode(^value, unknown: 1, canonical: true)",
        "JSON.decode(^value, depth: 1, depth: 2)",
        "JSON.encode(^value, canonical: true, canonical: false)",
        "(1 ..= 3).by(^1, step: 2)",
        "(1 ..= 3).by(step: ^1, step: 2)",
        "[1].reduce(^1, 2, 3)",
        "[1].count(^1, 2)",
        "using(^resource, callback) { ||; 1 }",
        "JSON.decode(arg1: ^value)",
        "[1].count(callback: ^value)",
        "print(arg1: ^value)",
        "JSON.decode(value, limits: ^2)",
        "JSON.decode(value, ^2)",
    ] {
        let given = format!("module Main {{ {call} }}");

        let when = help(&given);

        assert_eq!(when, None, "{call}");
    }
}

#[test]
fn retains_plausible_alternatives_when_call_is_incomplete() {
    for (call, lengths) in [
        ("[1].reduce(^", vec![1, 2]),
        ("[1].reduce(callback^", vec![1, 2]),
        ("[1].count(^", vec![0, 1]),
        ("(1 ..= 3).by(^", vec![1, 1]),
        ("JSON.encode(^", vec![1, 2]),
        ("using(resource^", vec![2, 2]),
    ] {
        let given = format!("module Main {{ {call}");

        let when = help(&given).unwrap();

        assert_eq!(
            when.signatures
                .iter()
                .map(|signature| signature.parameters.len())
                .collect::<Vec<_>>(),
            lengths,
            "{call}"
        );
        assert_eq!(when.active_signature, 0, "{call}");
    }
}

#[test]
fn rejects_missing_required_slots_when_call_is_complete() {
    for call in [
        "[1].reduce(^)",
        "(1 ..= 3).by(^)",
        "using(resource^)",
        "JSON.decode(^)",
    ] {
        let given = format!("module Main {{ {call} }}");

        let when = help(&given);

        assert_eq!(when, None, "{call}");
    }
}

#[test]
fn accepts_zero_or_repeated_arguments_when_rest_is_optional() {
    for (call, active) in [
        ("print(^)", Some(0)),
        ("print(^", Some(0)),
        ("print(1, 2, ^3)", Some(0)),
        ("[1].count(^)", None),
    ] {
        let given = format!("module Main {{ {call}");

        let when = help(&given).unwrap();

        assert_eq!(when.signatures.len(), 1, "{call}");
        assert_eq!(when.signatures[0].active_parameter, active, "{call}");
    }
}

#[test]
fn selects_keyword_shape_when_next_incomplete_slot_can_only_be_keyword() {
    let given = "module Main { JSON.decode(value, ^";

    let when = help(given).unwrap();

    assert_eq!(when.signatures.len(), 1);
    assert_eq!(when.signatures[0].active_parameter, Some(1));
    assert_eq!(
        when.signatures[0].parameters[1].category,
        ParameterCategory::Keyword
    );
}

#[test]
fn maps_next_slot_when_trailing_comma_precedes_existing_closer() {
    let given = "module Main { 'abc'.replace('a', ^) }";

    let when = help(given).unwrap();

    assert_eq!(when.signatures.len(), 1);
    assert_eq!(when.signatures[0].active_parameter, Some(1));
}

#[test]
fn rejects_excess_pending_argument_when_cursor_precedes_it() {
    let given = "module Main { [1].count(^callback, ";

    let when = help(given);

    assert_eq!(when, None);
}

#[test]
fn maps_external_block_when_parser_records_it_outside_parentheses() {
    let given = "module Main { using(resource,) { ||; 1 } }";

    let when = iris_parser::parse_editor(given);

    let call = &when.source.calls[0];
    assert_eq!(call.arguments.len(), 2);
    assert_eq!(call.commas.len(), 1);
    assert!(matches!(
        call.arguments[1].kind,
        ArgumentKind::TrailingBlock
    ));
    assert!(call.arguments[1].span.start > call.close.unwrap().end);
}

#[test]
fn selects_block_channel_without_highlighting_it_inside_parentheses() {
    let given = "module Main { using(resource,^ ) { ||; 1 } }";

    let when = help(given).unwrap();

    assert_eq!(when.signatures.len(), 1);
    assert_eq!(
        when.signatures[0].parameters[1].category,
        ParameterCategory::Block
    );
    assert_eq!(when.signatures[0].active_parameter, None);
}

#[test]
fn maps_resource_when_external_block_follows_cursor() {
    let given = "module Main { using(^resource) { ||; 1 } }";

    let when = help(given).unwrap();

    assert_eq!(when.signatures.len(), 1);
    assert_eq!(
        when.signatures[0].parameters[1].category,
        ParameterCategory::Block
    );
    assert_eq!(when.signatures[0].active_parameter, Some(0));
}

#[test]
fn avoids_callback_highlight_when_external_block_fills_positional_callback() {
    for call in [
        "[1].map(^) { |value|; value }",
        "[1].reduce(0,^ ) { |left, right|; left }",
    ] {
        let given = format!("module Main {{ {call} }}");

        let when = help(&given).unwrap();

        assert_eq!(when.signatures.len(), 1, "{call}");
        assert_eq!(when.signatures[0].active_parameter, None, "{call}");
    }
}

#[test]
fn preserves_backend_shapes_when_argument_types_differ() {
    for argument in ["1", "1.0", "value"] {
        let given = format!("module Main {{ Float64(^{argument}) }}");

        let when = help(&given).unwrap();

        assert_eq!(when.signatures.len(), 2, "{argument}");
        assert_eq!(when.active_signature, 0, "{argument}");
    }
}
