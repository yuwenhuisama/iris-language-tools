use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn hover_is_bounded_when_default_literal_is_oversized() {
    let text = format!(
        "module Main {{ fun read(value = '{}') {{ 'BODY' }} }}",
        "界".repeat(20_000)
    );
    let given = snapshot(&text);
    let when = given.hover(FileId(1), text.find("read").unwrap()).unwrap();
    assert!(when.signature.len() + when.type_label.as_ref().map_or(0, String::len) <= 4096);
    assert!(when.signature.ends_with("... [truncated]"));
    assert!(!when.signature.contains("BODY"));
}

#[test]
fn hover_is_bounded_when_written_type_is_oversized() {
    let name = "Type".repeat(5000);
    let text = format!("module Main {{ let value: {name} = nil; let copy = value }}");
    let given = snapshot(&text);
    for target in ["value:", "copy"] {
        let when = given.hover(FileId(1), text.find(target).unwrap()).unwrap();
        assert!(when.signature.len() + when.type_label.as_ref().map_or(0, String::len) <= 4096);
        assert!(when.signature.ends_with("... [truncated]"));
        assert!(when.type_label.unwrap().ends_with("... [truncated]"));
    }
}

#[test]
fn hover_keeps_literals_when_default_contains_header_delimiters() {
    let text = "module Main { fun read(value = 'a  ) { -> b', other = %{ :key: 'x' }) -> typeof(seed('}')) { 'BODY' } }";
    let given = snapshot(text);
    let when = given.hover(FileId(1), text.find("read").unwrap()).unwrap();
    assert_eq!(
        when.signature,
        "private fun read(value: Dynamic<Object> = 'a  ) { -> b', other: Dynamic<Object> = %{ :key: 'x' }) -> typeof(seed('}'))"
    );
    assert_eq!(when.type_label, None);
}

#[test]
fn hover_has_no_docs_when_comments_precede_declaration() {
    let text = "/// not reliable attachment\nclass Box {}";
    let given = snapshot(text);
    let when = given.hover(FileId(1), text.find("Box").unwrap()).unwrap();
    assert_eq!(when.signature, "class Box");
}

#[test]
fn prefix_hover_when_source_is_incomplete_never_panics() {
    let text = "class Box { public fun read(value = 'é') -> String { value } } module Main { let item = Box.new(); item.read() }";
    for end in (0..=text.len()).filter(|end| text.is_char_boundary(*end)) {
        let given = snapshot(&text[..end]);
        for byte in 0..=end + 1 {
            let when = given.hover(FileId(1), byte);
            if let Some(hover) = when {
                assert!(hover.span.start <= byte && byte < hover.span.end);
                assert!(text.is_char_boundary(byte));
            }
        }
    }
}

#[test]
fn inserted_annotation_is_separated_when_default_has_no_trivia() {
    let text = "module Main { fun read(value=1) {} }";
    let given = snapshot(text);
    let when = given.hover(FileId(1), text.find("read").unwrap()).unwrap();
    assert_eq!(
        when.signature,
        "private fun read(value: Dynamic<Object> =1) -> Dynamic<Object>"
    );
}

#[test]
fn default_literal_bytes_survive_when_multiline_and_raw() {
    for literal in ["'''a\n  b'''", "r'a\\b  c'", "'''a\r\nb'''"] {
        let text = format!("module Main {{ fun read(value = {literal}) {{}} }}");
        let given = snapshot(&text);
        let when = given
            .hover(FileId(1), text.find("read").unwrap())
            .unwrap_or_else(|| panic!("fixture: {literal:?}"));
        assert!(when.signature.contains(literal), "{}", when.signature);
    }
}
