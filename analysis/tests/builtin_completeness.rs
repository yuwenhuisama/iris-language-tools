use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([1, 2].map(|identity| SourceInput {
        id: FileId(identity),
        group: GroupId(identity),
        text: text.into(),
    }))
    .with_incomplete_groups([GroupId(1)])
}

#[test]
fn suppresses_catalog_completion_only_when_source_group_is_incomplete() {
    for (expression, label) in [("", "String"), ("'abc'.", "replace"), ("JSON.", "encode")] {
        let text = format!("module Main {{ {expression}");
        let given = snapshot(&text);

        let when = given.completions(FileId(1), text.len());

        assert!(
            !when.items.iter().any(|item| item.label == label),
            "{when:?}"
        );
        let complete = given.completions(FileId(2), text.len());
        assert!(
            complete.items.iter().any(|item| item.label == label),
            "{complete:?}"
        );
    }
}

#[test]
fn suppresses_catalog_hover_only_when_source_group_is_incomplete() {
    for (expression, selector) in [
        ("'abc'.replace('a', 'b')", "replace"),
        ("JSON.encode(1)", "encode"),
        ("print(1)", "print"),
        ("String", "String"),
    ] {
        let text = format!("module Main {{ {expression} }}");
        let given = snapshot(&text);
        let offset = text.find(selector).unwrap();

        let when = given.hover(FileId(1), offset);

        assert!(when.is_none(), "{when:?}");
        assert!(given.hover(FileId(2), offset).is_some());
    }
}

#[test]
fn suppresses_catalog_signature_only_when_source_group_is_incomplete() {
    for expression in ["'abc'.replace('a', 'b')", "JSON.encode(1)", "print(1)"] {
        let text = format!("module Main {{ {expression} }}");
        let given = snapshot(&text);
        let offset = text.find('(').unwrap() + 1;

        let when = given.signature_help(FileId(1), offset);

        assert!(when.is_none(), "{when:?}");
        assert!(given.signature_help(FileId(2), offset).is_some());
    }
}
