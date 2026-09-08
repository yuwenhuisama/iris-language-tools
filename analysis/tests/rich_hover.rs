use iris_analysis::{AnalysisSnapshot, FileId, GroupId, HoverDetail, HoverKind, SourceInput};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn rich_hover_when_documented_method_has_semantic_owner() {
    let text = "module Outer {} class Outer::Box {\n/// First paragraph.\n///\n/// <script>hostile</script> [run](command:run)\npublic fun read(value) -> String { value } }";
    let given = snapshot(text);
    let when = given.hover(FileId(1), text.find("read").unwrap()).unwrap();
    assert_eq!(when.kind, HoverKind::Method);
    assert_eq!(when.owner.as_deref(), Some("Outer::Box"));
    assert_eq!(when.details, vec![HoverDetail::ReturnType("String".into())]);
    assert_eq!(
        when.docs.unwrap().text,
        "First paragraph.\n\n<script>hostile</script> [run](command:run)"
    );
}

#[test]
fn docs_are_absent_when_attachment_is_broken() {
    for gap in ["\n", "// ordinary\n", "let other = 1;\n"] {
        let text = format!("module Main {{\n/// Detached\n{gap}let value = 1; value }}");
        let given = snapshot(&text);
        let when = given
            .hover(FileId(1), text.rfind("value").unwrap())
            .unwrap();
        assert_eq!(when.docs, None);
    }
}

#[test]
fn closure_parameter_stays_unknown_when_body_contains_types() {
    let text = "module Main { let block = { |arg|; let local: String; arg }; block }";
    let given = snapshot(text);
    let when = given.hover(FileId(1), text.rfind("arg").unwrap()).unwrap();
    assert_eq!(when.signature, "arg");
    assert_eq!(when.type_label, None);
}

#[test]
fn docs_are_bounded_when_unicode_text_exceeds_budget() {
    let text = format!("/// {}\nclass Box {{}}", "😀".repeat(2000));
    let given = snapshot(&text);
    let when = given.hover(FileId(1), text.find("Box").unwrap()).unwrap();
    let docs = when.docs.unwrap();
    assert!(docs.truncated);
    assert!(docs.text.len() <= 2048);
}

#[test]
fn rest_detail_when_signature_and_body_type_are_distinct() {
    let text = "module Main { fun read(*items: Integer, **options) { items; options } }";
    let given = snapshot(text);
    let when = given
        .hover(FileId(1), text.rfind("items").unwrap())
        .unwrap();
    assert_eq!(when.owner.as_deref(), Some("Main::read"));
    assert_eq!(
        when.details,
        vec![
            HoverDetail::ValueType("Array<Integer>".into()),
            HoverDetail::ParameterCategory(iris_analysis::ParameterCategory::Rest),
        ]
    );
    assert_eq!(when.signature, "*items: Integer");
}

#[test]
fn unknown_callable_value_when_closure_body_has_typed_locals() {
    let text = "module Main { let block = { |arg|; let local: Integer; 'result' }; block }";
    let given = snapshot(text);
    let when = given
        .hover(FileId(1), text.rfind("block").unwrap())
        .unwrap();
    assert_eq!(when.type_label, None);
    assert!(when.details.is_empty());
}

#[test]
fn block_docs_when_paragraphs_and_hostile_markup_are_present() {
    let text = "/**\n * Summary\n *\n * ```iris\n * <b>raw</b>\n */\nclass Box {}";
    let given = snapshot(text);
    let when = given.hover(FileId(1), text.find("Box").unwrap()).unwrap();
    let docs = when.docs.unwrap();
    assert_eq!(docs.text, "Summary\n\n```iris\n<b>raw</b>");
    assert!(!docs.truncated);
}

#[test]
fn decorated_docs_when_parser_attaches_to_method() {
    let text = "class Box {\n/// Decorated\n@Stamp()\npublic fun read() {} }";
    let given = snapshot(text);
    let when = given.hover(FileId(1), text.find("read").unwrap()).unwrap();
    assert_eq!(when.docs.unwrap().text, "Decorated");
}
