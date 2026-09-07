use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput, Span};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn hover_follows_shadow_when_closure_parameter_shadows_local() {
    let text = "module Main { let value = 1; let block = { |value: String|; value }; value }";
    let given = snapshot(text);
    let when = given
        .hover(FileId(1), text.find("value }").unwrap())
        .unwrap();
    assert_eq!(when.signature, "value: String");
    assert_eq!(when.type_label.as_deref(), Some("String"));
    assert_eq!(
        given
            .hover(FileId(1), text.rfind("value").unwrap())
            .unwrap()
            .type_label
            .as_deref(),
        Some("Integer")
    );
}

#[test]
fn alias_hover_when_target_is_in_another_file() {
    let caller = "from Core import Thing as Local\nmodule Main { let item: Local = Local.new() }";
    let given = AnalysisSnapshot::new([
        SourceInput {
            id: FileId(1),
            group: GroupId(1),
            text: caller.into(),
        },
        SourceInput {
            id: FileId(2),
            group: GroupId(1),
            text: "module Core {} class Core::Thing {}".into(),
        },
    ]);
    for start in caller.match_indices("Local").map(|(start, _)| start) {
        let when = given.hover(FileId(1), start).unwrap();
        assert_eq!(
            when.span,
            Span {
                start,
                end: start + 5
            }
        );
        assert_eq!(when.signature, "class Core::Thing");
        assert_eq!(when.type_label, None);
    }
}

#[test]
fn member_hover_when_receiver_identifies_owner() {
    let text = "class First { public fun read() -> Integer { 1 } } class Second { public fun read() -> String { 's' } } module Main { let item = First.new(); item.read() }";
    let given = snapshot(text);
    let when = given.hover(FileId(1), text.rfind("read").unwrap()).unwrap();
    assert_eq!(when.signature, "public fun read() -> Integer");
    assert_eq!(when.type_label.as_deref(), Some("Integer"));
}

#[test]
fn hover_is_absent_when_resolution_is_ambiguous_or_unsafe() {
    for (text, name) in [
        (
            "module Main { let value = 1; let value = 2; value }",
            "value",
        ),
        ("class Box {} class Box {} module Main { Box }", "Box"),
        (
            "class Box { public fun read() {} } module Main { fun use(item) { item.read() } }",
            "read",
        ),
        (
            "module Main { let value = 1; let broken = ; value }",
            "value",
        ),
        ("module Main { let value = 1; 'value' }", "value"),
        ("module Main { let value = 1; // value\n}", "value"),
    ] {
        let given = snapshot(text);
        let when = given.hover(FileId(1), text.rfind(name).unwrap());
        assert_eq!(when, None, "{text}");
    }
}

#[test]
fn unicode_hover_when_cursor_is_on_a_character_boundary() {
    let text = "module Main { let café = 1; café }";
    let given = snapshot(text);
    let start = text.rfind("café").unwrap();
    let when = given.hover(FileId(1), start + 3).unwrap();
    assert_eq!(
        when.span,
        Span {
            start,
            end: start + "café".len()
        }
    );
    for byte in [start + 4, start + 5, text.len(), text.len() + 1, usize::MAX] {
        assert_eq!(given.hover(FileId(1), byte), None);
    }
    assert_eq!(given.hover(FileId(999), 0), None);
}

#[test]
fn inferred_hover_when_binding_copies_known_types() {
    for (value, expected) in [
        ("'a  b'", Some("String")),
        ("1.0f32", Some("Float32")),
        ("Box.new()", Some("Box")),
        ("read()", Some("String")),
        ("1 + 2", None),
        ("missing()", None),
        ("Box", None),
    ] {
        let text = format!(
            "class Box {{}} module Main {{ fun read() -> String {{ 'yes' }}; let value = {value}; let copy = value; copy }}"
        );
        let given = snapshot(&text);
        let when = given.hover(FileId(1), text.rfind("copy").unwrap()).unwrap();
        assert_eq!(when.type_label.as_deref(), expected, "{value}");
    }
}

#[test]
fn written_hover_when_initializer_is_narrower() {
    let text = "module Main { let value: Object = 1; let copy = value }";
    let given = snapshot(text);
    let when = given.hover(FileId(1), text.find("copy").unwrap()).unwrap();
    assert_eq!(when.signature, "let copy: Object");
    assert_eq!(when.type_label.as_deref(), Some("Object"));
}

#[test]
fn method_value_hover_when_member_is_not_called_has_no_return_type() {
    let text = "class Box { public fun read() -> Integer { 1 } } module Main { let item = Box.new(); let value = item.read; value }";
    let given = snapshot(text);
    let when = given
        .hover(FileId(1), text.rfind("value").unwrap())
        .unwrap();
    assert_eq!(when.type_label, None);
}
