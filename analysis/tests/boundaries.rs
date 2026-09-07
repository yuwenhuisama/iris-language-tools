use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput, Span};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn namespace_completion_when_qualified_name_is_partial() {
    let text = "module Core {} class Core::Thing {} module Main { Core::Th }";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.rfind("Th }").unwrap() + 2);
    assert_eq!(
        when.items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["Thing"]
    );
}

#[test]
fn type_sites_resolve_independently_when_union_has_two_names() {
    let text = "class First {} class Second {} module Main { let item: First | Second = nil }";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.rfind("Second").unwrap());
    assert_eq!(when[0].name_span.start, text.find("Second").unwrap());
}

#[test]
fn written_type_is_preserved_when_initializer_is_narrower() {
    let text = "module Main { let value: Object = 1; let copy = value }";
    let given = snapshot(text);
    let when = given.inlay_hints(
        FileId(1),
        Span {
            start: 0,
            end: text.len(),
        },
    );
    assert_eq!(when.len(), 1);
    assert_eq!(when[0].label, ": Object");
}

#[test]
fn hints_use_float_width_when_literals_have_suffixes() {
    let text = "module Main { let first = 1.0f32; let second = 1.0f64; let plain = 1.0 }";
    let given = snapshot(text);
    let when = given.inlay_hints(
        FileId(1),
        Span {
            start: 0,
            end: text.len(),
        },
    );
    assert_eq!(
        when.iter()
            .map(|hint| hint.label.as_str())
            .collect::<Vec<_>>(),
        [": Float32", ": Float64", ": Float64"]
    );
}

#[test]
fn annotated_call_result_when_method_has_written_return() {
    let text = "module Main { fun read() -> String { 'yes' }; let result = read() }";
    let given = snapshot(text);
    let when = given.inlay_hints(
        FileId(1),
        Span {
            start: 0,
            end: text.len(),
        },
    );
    assert_eq!(when[0].label, ": String");
}

#[test]
fn unknown_operator_has_no_hint_when_result_is_not_modeled() {
    let text = "module Main { let result = 1 + 2 }";
    let given = snapshot(text);
    let when = given.inlay_hints(
        FileId(1),
        Span {
            start: 0,
            end: text.len(),
        },
    );
    assert!(when.is_empty());
}

#[test]
fn parameter_default_reads_earlier_parameter_when_visible() {
    let text = "module Main { fun read(first = 1, second = first) { second } }";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.rfind("first").unwrap());
    assert_eq!(when[0].name_span.start, text.find("first").unwrap());
}

#[test]
fn repaired_snapshot_retargets_when_target_file_changes() {
    let caller = "import Core as Alias\nmodule Main { Alias }";
    let given = AnalysisSnapshot::new([
        SourceInput {
            id: FileId(1),
            group: GroupId(1),
            text: caller.into(),
        },
        SourceInput {
            id: FileId(2),
            group: GroupId(1),
            text: "\n\nmodule Core {}".into(),
        },
    ]);
    let when = given.definitions(FileId(1), caller.rfind("Alias").unwrap());
    assert_eq!(when[0].name_span.start, 9);
    assert_eq!(when[0].file, FileId(2));
}

#[test]
fn foreign_group_is_not_resolved_when_name_matches() {
    let text = "import Core as Alias\nmodule Main { Alias }";
    let given = AnalysisSnapshot::new([
        SourceInput {
            id: FileId(1),
            group: GroupId(1),
            text: text.into(),
        },
        SourceInput {
            id: FileId(2),
            group: GroupId(2),
            text: "module Core {}".into(),
        },
    ]);
    let when = given.definitions(FileId(1), text.rfind("Alias").unwrap());
    assert!(when.is_empty());
}

#[test]
fn dotted_package_is_not_namespace_when_spellings_match() {
    let text = "module org {} module org::dep {} module org::dep::Core {} import org.dep::Core as Alias\nmodule Main { Alias }";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.rfind("Alias").unwrap());
    assert!(when.is_empty());
}

#[test]
fn unsupported_composition_suppresses_members_when_header_has_base() {
    let text = "class Base {} class Box extends Base { public fun read() {} } module Main { let item = Box.new(); item. }";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.rfind("item.").unwrap() + 5);
    assert!(when.items.is_empty());
}

#[test]
fn type_parameter_shadowing_when_method_declares_own_parameter() {
    let text = "class Box<T> { public fun read<T>(value: T) -> T { value } }";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.find("-> T").unwrap() + 3);
    assert_eq!(when[0].name_span.start, text.find("read<T>").unwrap() + 5);
}

#[test]
fn duplicate_receiver_owner_suppresses_members_when_two_origins_exist() {
    let text = "class Box { public fun read() {} } class Box {} module Main { let item = Box.new(); item. }";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.rfind("item.").unwrap() + 5);
    assert!(when.items.is_empty());
}

#[test]
fn namespace_declaration_resolves_when_used_inside_its_owner() {
    let text = "class Core::Thing {} module Core { let value: Thing = Thing.new() }";
    let given = snapshot(text);
    let when = given.definitions(FileId(1), text.rfind("Thing").unwrap());
    assert_eq!(when[0].name_span.start, text.find("Thing").unwrap());
}

#[test]
fn namespace_child_is_absent_when_completion_is_unqualified() {
    let text = "module Core {} class Core::Thing {} module Main { Th }";
    let given = snapshot(text);
    let when = given.completions(FileId(1), text.rfind("Th }").unwrap() + 2);
    assert!(when.items.is_empty());
}
