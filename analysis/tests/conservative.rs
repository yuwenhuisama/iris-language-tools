use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput, Span};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn members_distinguish_class_surface_when_receiver_is_class_object() {
    let text =
        "class Box { public fun read() {} public class fun build() {} } module Main { Box. }";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.rfind("Box.").unwrap() + 4);
    assert_eq!(
        when.items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["build"]
    );
}

#[test]
fn method_references_exclude_other_owner_when_selectors_match() {
    let text = "class First { public fun read() {} } class Second { public fun read() {} } module Main { First.new().read(); Second.new().read() }";
    let given = snapshot(text);
    let when = given.references(FileId(1), text.find("read").unwrap(), false);
    assert_eq!(when.len(), 1);
    assert_eq!(when[0].span.start, text.find(".read").unwrap() + 1);
}

#[test]
fn prefix_keeps_definition_when_later_scope_closer_is_missing() {
    let text = "module Main { let value = 1; value";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.rfind("value").unwrap());
    assert_eq!(when[0].name_span.start, text.find("value").unwrap());
}

#[test]
fn duplicate_local_blocks_implicit_send_when_selector_matches() {
    let text = "module Main { fun read() {} let read = 1; let read = 2; read() }";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.rfind("read").unwrap());
    assert!(when.is_empty());
}

#[test]
fn literal_completion_is_empty_when_query_is_numeric_or_symbol() {
    for value in ["123", ":value", "true", "nil"] {
        let text = format!("module Main {{ let value = 1; {value} }}");
        let given = snapshot(&text);
        let when = given.completions(FileId(1), text.rfind(value).unwrap() + value.len());
        assert!(when.items.is_empty(), "{value}: {when:?}");
    }
}

#[test]
fn byte_text_hints_when_literals_are_distinct_mutability_kinds() {
    let text = "module Main { let text = 's'; let mutable = m's'; let bytes = b's'; let array = mb's'; let symbol = :yes; let truth = true; let nothing = nil }";
    let given = snapshot(text);
    let when = given.inlay_hints(
        FileId(1),
        Span {
            start: 0,
            end: text.len(),
        },
    );
    assert_eq!(
        when.iter()
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        [
            ": String",
            ": MutableString",
            ": Bytes",
            ": ByteArray",
            ": Symbol",
            ": Bool",
            ": Nil"
        ]
    );
}

#[test]
fn conditional_member_is_absent_when_declaration_is_dynamic_only() {
    let text = "class Box { if true { public fun hidden() {} } public fun shown() {} } module Main { Box.new(). }";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.find("new().").unwrap() + 6);
    assert_eq!(
        when.items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["shown"]
    );
}

#[test]
fn comments_suppress_completions_when_at_eof() {
    let text = "module Main { let value = 1; # val";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.len());
    assert!(when.items.is_empty());
}

#[test]
fn parameter_default_never_narrows_when_annotation_omitted() {
    let text =
        "class Box { public fun read() {} } module Main { fun use(item = Box.new()) { item. } }";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.find("item. }").unwrap() + 5);
    assert!(when.items.is_empty());
}

#[test]
fn nested_default_return_hint_uses_outer_closer_when_calls_are_nested() {
    let text = "module Main { fun read(value = helper(inner(1))) { value } }";
    let given = snapshot(text);
    let when = given.inlay_hints(
        FileId(1),
        Span {
            start: 0,
            end: text.len(),
        },
    );
    assert!(
        when.iter()
            .any(|hint| hint.offset == text.find(" { value").unwrap()
                && hint.label == " -> Dynamic<Object>")
    );
}
