use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput, Span};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn members_resolve_when_receivers_have_distinct_owners() {
    let text = "class First { public fun read() -> Integer { 1 } } class Second { public fun read() -> String { 's' } } module Main { let item = First.new(); item.read() }";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.rfind("read").unwrap());
    assert_eq!(when[0].name_span.start, text.find("read").unwrap());
}

#[test]
fn member_completion_replaces_partial_identifier_when_receiver_known() {
    let text = "class Box { public fun read() {} private fun secret() {} public class fun build() {} } module Main { let item = Box.new(); item.re }";
    let given = snapshot(text);
    let start = text.rfind("re }").unwrap();
    let when = given.completions(FileId(1), start + 2);
    assert_eq!(
        when.items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["read"]
    );
    assert_eq!(
        when.items[0].replace,
        Span {
            start,
            end: start + 2
        }
    );
}

#[test]
fn hints_preserve_default_contract_when_method_annotations_omitted() {
    let text = "module Main { fun read(value = 1) { 2 }; let local = 1; let copy = local; let result = read() }";
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
            .any(|hint| hint.offset == text.find("value").unwrap() + 5
                && hint.label == ": Dynamic<Object>")
    );
    assert!(
        when.iter()
            .any(|hint| hint.offset == text.find(')').unwrap() + 1
                && hint.label == " -> Dynamic<Object>")
    );
    assert!(
        when.iter()
            .any(|hint| hint.offset == text.find("copy").unwrap() + 4 && hint.label == ": Integer")
    );
    assert!(
        !when.iter().any(
            |hint| hint.offset == text.find("result").unwrap() + 6 && hint.label == ": Integer"
        )
    );
}

#[test]
fn member_completion_is_empty_when_receiver_dynamic() {
    let text = "class Box { public fun read() {} } module Main { fun use(item) { item. } }";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.find("item. }").unwrap() + 5);
    assert!(when.items.is_empty());
}

#[test]
fn trailing_dot_keeps_known_members_when_owner_body_has_missing_brace() {
    let text = "class Box { public fun read() {} } module Main { let item = Box.new(); item.";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.len());
    assert_eq!(
        when.items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["read"]
    );
    assert!(when.is_incomplete);
}

#[test]
fn completion_skips_protected_text_when_inside_literal() {
    let text = "module Main { let visible = 1; let text = 'vis' }";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.find("vis'").unwrap() + 3);
    assert!(when.items.is_empty());
}

#[test]
fn imports_follow_explicit_alias_when_target_is_in_same_group() {
    let target = "module Core {} class Core::Thing {}";
    let text = "from Core import Thing as Local\nmodule Main { let item: Local = Local.new() }";
    let given = AnalysisSnapshot::new([
        SourceInput {
            id: FileId(1),
            group: GroupId(1),
            text: text.into(),
        },
        SourceInput {
            id: FileId(2),
            group: GroupId(1),
            text: target.into(),
        },
    ]);
    let when = given.definitions(FileId(1), text.rfind("Local").unwrap());
    assert_eq!(when[0].file, FileId(2));
    assert_eq!(when[0].name_span.start, target.find("Thing").unwrap());
}
