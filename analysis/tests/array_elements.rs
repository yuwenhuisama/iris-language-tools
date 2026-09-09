use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn infers_nullable_element_when_indexing_split_result() {
    let text = "let a=\"ffff\".split(\"\");let b=a[0];";
    let given = snapshot(text);

    let when = ["a=", "b="].map(|name| {
        given
            .hover(FileId(1), text.find(name).unwrap())
            .unwrap()
            .type_label
    });

    assert_eq!(when, [Some("Array<String>".into()), Some("String?".into())]);
}

#[test]
fn preserves_structural_facts_when_copying_or_slicing_arrays() {
    for (setup, expression, expected) in [
        ("let a='abc'.split(''); let copy=a;", "copy[0]", "String?"),
        ("let a='abc'.split(''); let copy=(a);", "copy[0]", "String?"),
        ("let a: Array<String> = [];", "a[0]", "String?"),
        ("let a: Array<Integer> = [];", "a[0]", "Integer?"),
        ("let a='abc'.split(''); let index=0;", "a[index]", "String?"),
        ("let a='abc'.split('');", "a[0 ..< 2]", "Array<String>"),
        (
            "let a='abc'.split(''); let part=a[0 ..< 2];",
            "part[0]",
            "String?",
        ),
        ("let a='abc'.split(''); let item=a[0];", "item", "String?"),
        ("", "'abc'.chars()[0]", "String?"),
        ("", "'abc'.to_array()[0]", "String?"),
        ("", "'abc'.graphemes()[0]", "String?"),
    ] {
        let text = format!("{setup} let result={expression};");
        let given = snapshot(&text);

        let when = given
            .hover(FileId(1), text.find("result=").unwrap())
            .unwrap();

        assert_eq!(when.type_label.as_deref(), Some(expected), "{text}");
    }
}

#[test]
fn refuses_element_facts_when_identity_or_contents_are_uncertain() {
    for (setup, expression) in [
        ("let a: Dynamic<Array<String>> = [];", "a[0]"),
        ("let a: Array<Dynamic<String>> = [];", "a[0]"),
        ("let a=[];", "a[0]"),
        ("let a='abc'.split('');", "a[unknown]"),
        ("let a='abc'.split('');", "a['key']"),
        ("mut a='abc'.split('');", "a[0]"),
        ("let a='abc'.split(''); a[0]=1;", "a[0]"),
        ("let a='abc'.split(''); (a[0])=1;", "a[0]"),
        ("let a='abc'.split(''); a.push(1);", "a[0]"),
        ("let a='abc'.split(''); let copy=a; copy.push(1);", "a[0]"),
        ("let a='abc'.split(''); mut copy=a;", "a[0]"),
        ("let a='abc'.split(''); unknown(a);", "a[0]"),
        ("let a='abc'.split(''); let stored=[a];", "a[0]"),
        ("let a='abc'.split(''); let closure={ a };", "a[0]"),
        ("class String {} let a: Array<String> = [];", "a[0]"),
        ("class Array<T> {} let a: Array<String> = [];", "a[0]"),
        ("open class String {} let a='abc'.split('');", "a[0]"),
        ("open class Array {} let a='abc'.split('');", "a[0]"),
        ("let a=Object.new().to_string;", "a.split('')[0]"),
    ] {
        let text = format!("{setup} let result={expression};");
        let given = snapshot(&text);

        let when = given.hover(FileId(1), text.find("result=").unwrap());

        assert!(when.and_then(|hover| hover.type_label).is_none(), "{text}");
    }
}

#[test]
fn refuses_array_inference_when_inventory_is_incomplete() {
    let text = "let a=\"ffff\".split(\"\");let b=a[0];";
    let given = snapshot(text).with_incomplete_groups([GroupId(1)]);

    let when = given.inlay_hints(
        FileId(1),
        iris_analysis::Span {
            start: 0,
            end: text.len(),
        },
    );

    assert!(when.is_empty());
}

#[test]
fn refuses_string_completion_when_index_result_is_nullable() {
    let text = "let a='abc'.split(''); let b=a[0]; b.";
    let given = snapshot(text);

    let when = given.completions(FileId(1), text.len());

    assert!(when.items.is_empty());
}

#[test]
fn emits_structural_hints_when_indexing_split_result() {
    let text = "let a=\"ffff\".split(\"\");let b=a[0];";
    let given = snapshot(text);

    let when = given.inlay_hints(
        FileId(1),
        iris_analysis::Span {
            start: 0,
            end: text.len(),
        },
    );

    assert_eq!(
        when.iter()
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        [": Array<String>", ": String?"]
    );
}

#[test]
fn refuses_element_facts_when_constants_expose_arrays_to_other_files() {
    for setup in [
        "const Parts='ffff'.split(''); let b=Parts[0];",
        "let a='ffff'.split(''); const Parts=a; let b=a[0];",
    ] {
        let text = format!("module Core {{ {setup} }}");
        let given = AnalysisSnapshot::new([
            SourceInput {
                id: FileId(1),
                group: GroupId(1),
                text: text.clone().into(),
            },
            SourceInput {
                id: FileId(2),
                group: GroupId(1),
                text: "module Other { Core::Parts.push(1); }".into(),
            },
        ]);

        let when = given.hover(FileId(1), text.find("b=").unwrap()).unwrap();

        assert_eq!(when.type_label, None, "{text}");
    }
}
