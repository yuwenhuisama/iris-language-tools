use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput, Span};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn local_hover_when_declaration_and_use_share_identity() {
    let text = "module Main { let value = 1; value }";
    let given = snapshot(text);
    for start in [text.find("value").unwrap(), text.rfind("value").unwrap()] {
        let when = given.hover(FileId(1), start).unwrap();
        assert_eq!(
            when.span,
            Span {
                start,
                end: start + 5
            }
        );
        assert_eq!(when.signature, "let value: Integer");
        assert_eq!(when.type_label.as_deref(), Some("Integer"));
    }
}

#[test]
fn hover_is_absent_when_cursor_is_after_name() {
    let text = "module Main { let value = 1; value }";
    let given = snapshot(text);
    let when = given.hover(FileId(1), text.rfind("value").unwrap() + 5);
    assert_eq!(when, None);
    assert_eq!(
        given
            .definitions(FileId(1), text.rfind("value").unwrap() + 5)
            .len(),
        1
    );
}

#[test]
fn method_hover_when_discard_and_default_parameters_are_present() {
    let text = "module Main { public async module fun read<T>(_, _: Integer = 1, value = 'a  ) { b', *rest: T, key option = true, **kwargs, &block) -> String { 'BODY' } }";
    let given = snapshot(text);
    let when = given.hover(FileId(1), text.find("read").unwrap()).unwrap();
    assert_eq!(
        when.signature,
        "public async module fun read<T>(_: Dynamic<Object>, _: Integer = 1, value: Dynamic<Object> = 'a  ) { b', *rest: T, key option: Dynamic<Object> = true, **kwargs: Dynamic<Object>, &block: Dynamic<Object>) -> String"
    );
    assert_eq!(when.type_label.as_deref(), Some("String"));
}

#[test]
fn method_hover_when_annotations_are_omitted() {
    let text = "module Main { fun read(value = 1) { 2 } }";
    let given = snapshot(text);
    let when = given.hover(FileId(1), text.find("read").unwrap()).unwrap();
    assert_eq!(
        when.signature,
        "private fun read(value: Dynamic<Object> = 1) -> Dynamic<Object>"
    );
    assert_eq!(when.type_label.as_deref(), Some("Dynamic<Object>"));
}

#[test]
fn nominal_hover_when_header_contains_composition_and_generics() {
    for header in [
        "class Box<T> extends Base",
        "module Mix<T> for Contract",
        "contract Readable<T>",
    ] {
        let text = format!("{header} {{}}");
        let given = snapshot(&text);
        let start = text.find(' ').unwrap() + 1;
        let when = given.hover(FileId(1), start).unwrap();
        assert_eq!(when.signature, header);
        assert_eq!(when.type_label, None);
    }
}

#[test]
fn rest_hover_when_body_type_differs_from_signature_annotation() {
    let text = "module Main { fun read(*values: Integer, **options) { values; options } }";
    let given = snapshot(text);
    for (name, signature, label) in [
        ("values", "*values: Integer", "Array<Integer>"),
        (
            "options",
            "**options: Dynamic<Object>",
            "Hash<Symbol, Dynamic<Object>>",
        ),
    ] {
        let when = given.hover(FileId(1), text.rfind(name).unwrap()).unwrap();
        assert_eq!(when.signature, signature);
        assert_eq!(when.type_label.as_deref(), Some(label));
    }
}

#[test]
fn written_type_hover_when_typeof_is_not_evaluated() {
    let text = "module Main { let value: typeof(seed()) = 1; value }";
    let given = snapshot(text);
    let when = given
        .hover(FileId(1), text.rfind("value").unwrap())
        .unwrap();
    assert_eq!(when.signature, "let value: typeof(seed())");
    assert_eq!(when.type_label, None);
}

#[test]
fn declaration_hover_when_binding_has_source_modifiers() {
    for (source, target, expected) in [
        (
            "module Main { mut value = 1 }",
            "value",
            "mut value: Integer",
        ),
        (
            "module Main { const Value = 's' }",
            "Value",
            "const Value: String",
        ),
        ("type Text = String", "Text", "type Text = String"),
        (
            "class Box { public property value: Integer { public get; } }",
            "value",
            "public property value: Integer",
        ),
        (
            "contract Reader { public fun read(value) }",
            "read",
            "public fun read(value: Dynamic<Object>) -> Dynamic<Object>",
        ),
        (
            "class Box { override public class fun read<T>(value: T) -> T { value } }",
            "read",
            "override public class fun read<T>(value: T) -> T",
        ),
    ] {
        let given = snapshot(source);
        let when = given
            .hover(FileId(1), source.find(target).unwrap())
            .unwrap();
        assert_eq!(when.signature, expected);
    }
}
