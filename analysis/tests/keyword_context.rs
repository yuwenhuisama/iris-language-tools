use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn keywords_allowed_when_cursor_is_in_ordinary_code() {
    for (text, byte) in [
        ("let value = 1", 0),
        ("", 0),
        ("ret", 3),
        ("module Main { ret", 17),
        ("module Main {", 13),
        ("let value = 1;", 14),
    ] {
        let given = snapshot(text);
        let when = given.completions(FileId(1), byte);
        assert!(when.allow_keywords, "{text:?}: {when:?}");
    }
}

#[test]
fn keywords_suppressed_when_context_is_not_ordinary_code() {
    for text in [
        "unknown.",
        "unknown.re",
        "Core::",
        "Core::Th",
        "123",
        ":name",
        "true",
        "nil",
        "// comment",
        "/* comment",
        "'unfinished",
        "module Main { let bad = ; ret",
    ] {
        let given = snapshot(text);
        let when = given.completions(FileId(1), text.len());
        assert!(!when.allow_keywords, "{text:?}: {when:?}");
    }
}

#[test]
fn safe_prefix_allows_keywords_when_later_lexer_exhausts_resources() {
    let text = format!("let x={}\"x\"{}", "\"${".repeat(10000), "}\"".repeat(10000));
    let given = snapshot(&text);
    let when = given.completions(FileId(1), 0);
    assert!(when.allow_keywords);
    assert!(when.items.is_empty());
    assert!(when.is_incomplete);
}

#[test]
fn damaged_lexical_suffix_does_not_poison_reliable_prefix() {
    for (text, byte) in [("ret 'unfinished", 3), ("ret /* unfinished", 3)] {
        let given = snapshot(text);
        let when = given.completions(FileId(1), byte);
        assert!(when.allow_keywords, "{text:?}");
    }
}

#[test]
fn damaged_literals_and_comments_remain_protected_when_prefix_is_relexed() {
    for text in [
        "let x = 'unfinished",
        "let x = 1; /* unfinished",
        "let x = 1; // comment",
        "let x = :symbol 'unfinished",
    ] {
        let given = snapshot(text);
        let when = given.completions(FileId(1), text.len());
        assert!(!when.allow_keywords, "{text:?}");
    }
}
