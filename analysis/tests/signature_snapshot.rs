use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn snapshot(target: &str, group: GroupId) -> (AnalysisSnapshot, String) {
    let caller =
        "from Core import Box as Local\nmodule Main { let item: Local = Local.new(); item.read("
            .to_owned();
    let snapshot = AnalysisSnapshot::new([
        SourceInput {
            id: FileId(1),
            group: GroupId(1),
            text: caller.clone().into(),
        },
        SourceInput {
            id: FileId(2),
            group,
            text: target.into(),
        },
    ]);
    (snapshot, caller)
}

#[test]
fn signature_and_hover_when_imported_target_changes_in_dirty_snapshot() {
    let first = "module Core {} class Core::Box {\n/// Original\npublic fun read(value: Integer) -> String {} }";
    let second = "module Core {} class Core::Box {\n/// Updated\npublic fun read(key option: String) -> Bool {} }";
    let (given, caller) = snapshot(first, GroupId(1));
    let (updated, _) = snapshot(second, GroupId(1));
    let when = updated
        .hover(FileId(1), caller.rfind("read").unwrap())
        .unwrap();
    assert_eq!(when.docs.unwrap().text, "Updated");
    assert_eq!(when.owner.as_deref(), Some("Core::Box"));
    assert_eq!(when.type_label.as_deref(), Some("Bool"));
    assert_eq!(updated.signature_help(FileId(1), caller.len()), None);
    let original = given.signature_help(FileId(1), caller.len()).unwrap();
    assert_eq!(original.signature.docs.unwrap().text, "Original");
    assert_eq!(original.signature.parameters[0].name, "value");
}

#[test]
fn signature_is_absent_when_import_target_is_in_another_group() {
    let (given, caller) = snapshot(
        "module Core {} class Core::Box { public fun read(value) {} }",
        GroupId(2),
    );
    let when = given.signature_help(FileId(1), caller.len());
    assert_eq!(when, None);
}

#[test]
fn alias_hover_when_docs_belong_to_target_not_import_site() {
    let target = "module Core {}\n/// Target docs\nclass Core::Box {}";
    let (given, caller) = snapshot(target, GroupId(1));
    let when = given
        .hover(FileId(1), caller.find("Local").unwrap())
        .unwrap();
    assert_eq!(when.owner.as_deref(), Some("Core"));
    assert_eq!(when.docs.unwrap().text, "Target docs");
}
