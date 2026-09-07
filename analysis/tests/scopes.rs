use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn initializer_reads_outer_when_inner_binding_shadows() {
    let text = "module Main { let value = 1; if true { let value = value; value } }";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.find("= value").unwrap() + 2);
    assert_eq!(when.len(), 1);
    assert_eq!(when[0].name_span.start, text.find("value").unwrap());
}

#[test]
fn references_distinguish_shadowed_symbols_when_names_repeat() {
    let text = "module Main { let value = 1; if true { let value = value; value }; value }";
    let given = snapshot(text);
    let when = given.references(FileId(1), text.find("value").unwrap(), false);
    assert_eq!(when.len(), 2);
    assert_eq!(when[0].span.start, text.find("= value").unwrap() + 2);
    assert_eq!(when[1].span.start, text.rfind("value").unwrap());
}

#[test]
fn method_does_not_capture_when_body_local_exists() {
    let text = "module Main { let value = 1; fun read() { value } }";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.rfind("value").unwrap());
    assert!(when.is_empty());
}

#[test]
fn closure_captures_when_outer_local_exists() {
    let text = "module Main { let value = 1; let block = { ||; value } }";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.rfind("value").unwrap());
    assert_eq!(when[0].name_span.start, text.find("value").unwrap());
}

#[test]
fn duplicate_bindings_are_ambiguous_when_same_scope_redeclares() {
    let text = "module Main { let value = 1; let value = 2; value }";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.rfind("value").unwrap());
    assert!(when.is_empty());
}

#[test]
fn type_parameter_resolves_when_used_in_method_signature() {
    let text = "class Box<T> { public fun read(value: T) -> T { value } }";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.find("-> T").unwrap() + 3);
    assert_eq!(when[0].name_span.start, text.find("<T").unwrap() + 1);
}
