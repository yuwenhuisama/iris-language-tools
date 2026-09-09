use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn container_completion_when_annotations_or_rest_bindings_prove_outer_family() {
    for (parameter, selector) in [
        ("value: Array<String>", "push"),
        ("value: Hash<Symbol, String>", "fetch"),
        ("*value: String", "push"),
        ("**value: String", "fetch"),
    ] {
        let text = format!("module Main {{ fun use({parameter}) {{ let copy = value; copy.");
        let given = snapshot(&text);
        let when = given.completions(FileId(1), text.len());
        assert!(
            when.items.iter().any(|item| item.label == selector),
            "{text}: {when:?}"
        );
        assert!(
            !when.items.iter().any(|item| item.label == "replace"),
            "{text}"
        );
    }
}

#[test]
fn signature_is_absent_when_source_candidates_block_builtin_fallback() {
    for text in [
        "module Main { fun print(value) {} fun print(other) {} print(",
        "fun print(value) {} fun print(other) {} print(",
        "from org.dep::Core import print\nmodule Main { print(",
        "module Main { let print = 1; print(",
        "module Main { let JSON = 1; JSON.encode(",
        "class String {} class String {} module Main { fun use(value: String) { value.replace(",
        "class Array<T> {} module Main { fun use(value: Array<String>) { value.push(",
        "module Main { fun use(value: Dynamic<String> = 'abc') { value.replace(",
        "module Main { 'abc'.replace('a', unknown(",
        "module Main { let callback = { |value|; value }; callback.call(",
    ] {
        let given = snapshot(text);
        let when = given.signature_help(FileId(1), text.len());
        assert_eq!(when, None, "{text}");
    }
}

#[test]
fn signature_candidates_when_argument_count_or_keyword_selects_shape() {
    for (call, count, parameter) in [
        ("[1].reduce(0, ", 1, 1),
        ("[1].count(", 2, 0),
        ("(1 ..= 3).by(step: ", 1, 0),
        ("JSON.encode(1, canonical: ", 1, 1),
    ] {
        let text = format!("module Main {{ {call}");
        let given = snapshot(&text);
        let when = given.signature_help(FileId(1), text.len()).unwrap();
        assert_eq!(when.signatures.len(), count, "{text}: {when:?}");
        let active = &when.signatures[when.active_signature];
        if call == "[1].count(" {
            assert!(active.parameters.is_empty());
            assert_eq!(active.active_parameter, None);
        } else {
            assert_eq!(active.active_parameter, Some(parameter), "{text}");
        }
    }
}

#[test]
fn positional_placeholder_is_not_a_keyword_when_catalog_names_display_slots() {
    let text = "module Main { 'abc'.replace(arg1: ";
    let given = snapshot(text);
    let when = given.signature_help(FileId(1), text.len());
    assert_eq!(when, None);
}

#[test]
fn prefix_replacement_when_selector_has_existing_suffix() {
    let text = "module Main { 'abc'.replace('a', 'b') }";
    let given = snapshot(text);
    let start = text.find("replace").unwrap();
    let when = given.completions(FileId(1), start + 2);
    let item = when
        .items
        .iter()
        .find(|item| item.label == "replace")
        .unwrap();
    assert_eq!(&text[item.replace.start..item.replace.end], "replace");
}

#[test]
fn source_class_metadata_when_selector_is_absent_and_owner_is_certain() {
    for (declaration, expected) in [
        ("class Box {}", true),
        ("class Box { public class fun define_method(own) {} }", true),
        (
            "class Box { private class fun define_method(own) {} }",
            false,
        ),
        (
            "class Box { public class fun define_method(first) {} public class fun define_method(second) {} }",
            false,
        ),
        ("class Box {} class Box {}", false),
    ] {
        let text = format!("{declaration} module Main {{ Box.define_method(");
        let given = snapshot(&text);
        let when = given.signature_help(FileId(1), text.len());
        assert_eq!(when.is_some(), expected, "{text}");
        if declaration.contains("own") && expected {
            assert_eq!(when.unwrap().signatures[0].parameters[0].name, "own");
        }
    }
}
