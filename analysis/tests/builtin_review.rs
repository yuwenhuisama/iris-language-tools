use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn metadata_is_available_when_receiver_is_builtin_class() {
    for name in ["Float64", "String"] {
        let text = format!("module Main {{ {name}.define_method(");
        let given = snapshot(&text);
        let when = given.signature_help(FileId(1), text.len()).unwrap();
        assert_eq!(when.signatures[0].parameters.len(), 2);
    }
}

#[test]
fn intrinsic_methods_are_absent_when_new_allocates_ordinary_object() {
    let text = "module Main { String.new().rep";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.len());
    assert!(when.items.is_empty());
}

#[test]
fn special_value_methods_are_available_when_transformation_is_unshadowed() {
    let text = "module Main { Transformation.add_method(";
    let given = snapshot(text);
    let when = given.signature_help(FileId(1), text.len()).unwrap();
    assert_eq!(when.signatures[0].parameters.len(), 2);
}

#[test]
fn special_value_name_is_available_when_completing_prefix() {
    let text = "module Main { Trans";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.len());
    assert!(when.items.iter().any(|item| item.label == "Transformation"));
}

#[test]
fn special_value_hover_is_available_when_name_is_used() {
    let text = "module Main { Transformation.empty() }";
    let given = snapshot(text);
    let when = given
        .hover(FileId(1), text.find("Transformation").unwrap())
        .unwrap();
    assert_eq!(when.type_label.as_deref(), Some("Transformation"));
}

#[test]
fn special_value_methods_are_absent_when_transformation_is_shadowed() {
    let text = "module Main { let Transformation = 1; Transformation.add_method(";
    let given = snapshot(text);
    let when = given.signature_help(FileId(1), text.len());
    assert_eq!(when, None);
}

#[test]
fn literal_methods_are_suppressed_when_same_group_reopens_builtin() {
    for group in [GroupId(1), GroupId(2)] {
        let text = "module Main { 'abc'.replace(";
        let given = AnalysisSnapshot::new([
            SourceInput {
                id: FileId(1),
                group: GroupId(1),
                text: text.into(),
            },
            SourceInput {
                id: FileId(2),
                group,
                text: "open class String { public fun replace(value) {} }".into(),
            },
        ]);
        let when = given.signature_help(FileId(1), text.len());
        assert_eq!(when.is_some(), group == GroupId(2));
    }
}

#[test]
fn closure_hover_exists_without_fixed_signature_when_callable_is_known() {
    for parameter in [
        "Closure<(Object) -> Object>",
        "BoundMethod<(Object) -> Object>",
    ] {
        let text = format!("module Main {{ fun use(value: {parameter}) {{ value.call(");
        let given = snapshot(&text);
        let when = given.hover(FileId(1), text.rfind("call").unwrap()).unwrap();
        assert!(when.signature.contains("call"));
        assert_eq!(given.signature_help(FileId(1), text.len()), None);
    }
}
