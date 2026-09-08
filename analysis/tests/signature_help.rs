use iris_analysis::{AnalysisSnapshot, FileId, GroupId, ParameterCategory, SourceInput};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn signature_when_parameters_include_discard_defaults_and_channels() {
    let text = "module Main {\n/// Read docs\npublic fun read<T>(_, value = '😀', *rest: T, key option = true, **kwargs, &block) -> String {}\nread(1, 2) }";
    let given = snapshot(text);
    let when = given
        .signature_help(FileId(1), text.rfind("2)").unwrap())
        .unwrap();
    assert_eq!(when.active_parameter, Some(1));
    assert_eq!(when.signature.docs.unwrap().text, "Read docs");
    assert_eq!(when.signature.parameters.len(), 6);
    for (parameter, expected) in when.signature.parameters.iter().zip([
        "_: Dynamic<Object>",
        "value: Dynamic<Object> = '😀'",
        "*rest: T",
        "key option: Dynamic<Object> = true",
        "**kwargs: Dynamic<Object>",
        "&block: Dynamic<Object>",
    ]) {
        assert_eq!(
            &when.signature.label[parameter.label.start..parameter.label.end],
            expected
        );
    }
    assert_eq!(
        when.signature.parameters[2].category,
        ParameterCategory::Rest
    );
    let hover = given
        .hover(FileId(1), text.find("read<T>").unwrap())
        .unwrap();
    assert_eq!(when.signature.label, hover.signature);
}

#[test]
fn active_parameter_when_keyword_and_rest_channels_are_mixed() {
    for (call, active) in [
        ("read(1, 2, 3", 1),
        ("read(option: 1", 2),
        ("read(other: 1", 3),
    ] {
        let text = format!(
            "module Main {{ public fun read(value, *rest, key option, **kwargs) {{}} {call}"
        );
        let given = snapshot(&text);
        let when = given.signature_help(FileId(1), text.len()).unwrap();
        assert_eq!(when.active_parameter, Some(active), "{call}");
    }
}

#[test]
fn signature_when_editor_call_is_incomplete() {
    for (call, active) in [
        ("read(", 0),
        ("read(1,", 1),
        ("read(option:", 2),
        ("read(option: 1 +", 2),
    ] {
        let text = format!("module Main {{ public fun read(value, second, key option) {{}} {call}");
        let given = snapshot(&text);
        let when = given.signature_help(FileId(1), text.len()).unwrap();
        assert_eq!(when.active_parameter, Some(active), "{call}");
    }
}

#[test]
fn innermost_call_when_outer_and_inner_are_incomplete() {
    let text = "module Main { public fun outer(a,b) {} public fun inner(value) {} outer(1, inner(";
    let given = snapshot(text);
    let when = given.signature_help(FileId(1), text.len()).unwrap();
    assert!(when.signature.label.contains("inner("));
    assert_eq!(when.active_parameter, Some(0));
}

#[test]
fn signature_is_absent_when_inner_callee_is_unknown() {
    let text = "module Main { public fun outer(a,b) {} outer(1, unknown(";
    let given = snapshot(text);
    let when = given.signature_help(FileId(1), text.len());
    assert_eq!(when, None);
}

#[test]
fn signature_has_no_active_parameter_when_method_has_no_parameters() {
    let text = "module Main { public fun read() {} read() }";
    let given = snapshot(text);
    let when = given
        .signature_help(FileId(1), text.rfind("read(").unwrap() + 5)
        .unwrap();
    assert_eq!(when.active_parameter, None);
    assert!(when.signature.parameters.is_empty());
}

#[test]
fn signature_is_absent_when_cursor_is_outside_arguments_or_mapping_is_impossible() {
    for call in ["read(1, 2", "read(other: 1", "read(1)"] {
        let text = format!("module Main {{ public fun read(value) {{}} {call}");
        let given = snapshot(&text);
        let when = given.signature_help(FileId(1), text.len());
        assert_eq!(when, None, "{call}");
    }
}

#[test]
fn outer_slot_when_nested_literals_and_comments_contain_commas() {
    let text = "module Main { public fun read(a,b,c) {} read([1,2], 'a,b', /* , */ 3) }";
    let given = snapshot(text);
    let when = given
        .signature_help(FileId(1), text.rfind("3)").unwrap())
        .unwrap();
    assert_eq!(when.active_parameter, Some(2));
}

#[test]
fn signature_is_absent_when_complete_parameter_structure_exceeds_budget() {
    let text = format!(
        "module Main {{ public fun read(value = '{}') {{}} read(",
        "😀".repeat(2000)
    );
    let given = snapshot(&text);
    let when = given.signature_help(FileId(1), text.len());
    assert_eq!(when, None);
}
