use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn completion_when_receiver_has_proven_builtin_identity() {
    for (setup, receiver, expected) in [
        ("", "'abc'", "replace"),
        ("let text = 'abc';", "text", "trim"),
        ("", "[1, 2]", "push"),
        ("", "(1, 2)", "to_array"),
        ("", "%{1: 2}", "fetch"),
        ("", "(1 ..= 3)", "by"),
        ("", "Float64", "from_bits"),
        ("let alias = Float64;", "alias", "from_bits"),
        ("", "Float64.nan", "to_bits"),
        ("", "JSON", "encode"),
        ("", "Encoding::UTF_8", "decode"),
        ("", "'abc'.split(',')", "push"),
        ("let callback = { |value|; value };", "callback", "call"),
    ] {
        let text = format!("module Main {{ {setup} {receiver}.");
        let given = snapshot(&text);
        let when = given.completions(FileId(1), text.len());
        assert!(
            when.items.iter().any(|item| item.label == expected),
            "{text}: {when:?}"
        );
    }
}

#[test]
fn class_only_property_is_absent_when_receiver_is_float_instance() {
    let text = "module Main { 1.0.na";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.len());
    assert!(when.items.is_empty());
}

#[test]
fn hover_when_builtin_selector_has_real_occurrence() {
    let text = "module Main { 'abc'.replace('a', 'b') }";
    let given = snapshot(text);
    let byte = text.find("replace").unwrap();
    let when = given.hover(FileId(1), byte).unwrap();
    assert_eq!(&text[when.span.start..when.span.end], "replace");
    assert_eq!(when.owner.as_deref(), Some("String"));
    assert_eq!(when.type_label.as_deref(), Some("String"));
    assert!(given.definitions(FileId(1), byte).is_empty());
    assert!(given.references(FileId(1), byte, true).is_empty());
}

#[test]
fn hover_return_evidence_is_not_a_declared_annotation_when_target_is_builtin() {
    let text = "module Main { 'abc'.replace('a', 'b') }";
    let given = snapshot(text);

    let when = given
        .hover(FileId(1), text.find("replace").unwrap())
        .unwrap();

    assert_eq!(when.type_label.as_deref(), Some("String"));
    assert!(when.details.is_empty());
    assert!(when.docs.is_some());
}

#[test]
fn signature_when_builtin_has_known_positional_shape() {
    let text = "module Main { 'abc'.replace('a', ";
    let given = snapshot(text);
    let when = given.signature_help(FileId(1), text.len()).unwrap();
    assert_eq!(
        when.signatures[when.active_signature].active_parameter,
        Some(1)
    );
    assert_eq!(when.signatures[0].parameters.len(), 2);
}

#[test]
fn completion_is_absent_when_identity_is_unknown_or_name_is_shadowed() {
    for (setup, receiver) in [
        ("let text: Dynamic<String> = 'abc';", "text"),
        ("mut text = 'abc';", "text"),
        ("let alias = JSON;", "alias"),
        ("", "[1].first()"),
        ("", "'abc'.replace"),
        ("", "Iteration.done()"),
    ] {
        let text = format!("module Main {{ {setup} {receiver}.re");
        let given = snapshot(&text);
        let when = given.completions(FileId(1), text.len());
        assert!(when.items.is_empty(), "{text}: {when:?}");
    }
}

#[test]
fn literal_identity_is_independent_when_source_class_has_same_name() {
    for (receiver, expected) in [("'abc'", true), ("text", false)] {
        let text =
            format!("class String {{}} module Main {{ let text: String = 'abc'; {receiver}.rep");
        let given = snapshot(&text);
        let when = given.completions(FileId(1), text.len());
        assert_eq!(
            when.items.iter().any(|item| item.label == "replace"),
            expected
        );
    }
}
