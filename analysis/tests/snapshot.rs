use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput, Span};

fn input(id: u32, text: &str) -> SourceInput {
    SourceInput {
        id: FileId(id),
        group: GroupId(1),
        text: text.into(),
    }
}

#[test]
fn four_features_rebuild_when_target_signature_changes() {
    let caller = "import Core as Alias\nmodule Main { let item = Alias::Box.new(); let result = item.read(); item.re }";
    let original = "module Core {} class Core::Box { public fun read() -> Integer { 1 } }";
    let updated = "\nmodule Core {} class Core::Box { public fun read() -> String { 'changed' } }";
    let given = AnalysisSnapshot::new([input(1, caller), input(2, original)]);
    let rebuilt = AnalysisSnapshot::new([input(1, caller), input(2, updated)]);
    let when = (
        rebuilt.definitions(FileId(1), caller.find("read").unwrap()),
        rebuilt.references(FileId(2), updated.find("read").unwrap(), true),
        rebuilt.completions(FileId(1), caller.rfind("re }").unwrap() + 2),
        rebuilt.inlay_hints(
            FileId(1),
            Span {
                start: 0,
                end: caller.len(),
            },
        ),
    );
    assert_eq!(when.0[0].name_span.start, updated.find("read").unwrap());
    assert_eq!(when.1.len(), 2);
    assert_eq!(when.2.items[0].label, "read");
    assert!(when.3.iter().any(|hint| hint.label == ": String"));
    assert!(
        given
            .inlay_hints(
                FileId(1),
                Span {
                    start: 0,
                    end: caller.len()
                }
            )
            .iter()
            .any(|hint| hint.label == ": Integer")
    );
}

#[test]
fn every_utf8_boundary_is_safe_when_source_has_partial_syntax() {
    let fixture = "class Box { public fun read(value: String) -> String { value } } module Main { let 名 = Box.new(); 名.read('值'); 名.";
    for end in fixture
        .char_indices()
        .map(|(offset, _)| offset)
        .chain([fixture.len()])
    {
        let text = &fixture[..end];
        let given = AnalysisSnapshot::new([input(1, text)]);
        let when = given.completions(FileId(1), end);
        for item in when.items {
            assert!(item.replace.start <= item.replace.end && item.replace.end <= text.len());
            assert!(text.is_char_boundary(item.replace.start));
            assert!(text.is_char_boundary(item.replace.end));
        }
        for hint in given.inlay_hints(FileId(1), Span { start: 0, end }) {
            assert!(text.is_char_boundary(hint.offset));
        }
    }
}

#[test]
fn unknown_file_and_invalid_byte_return_empty_when_host_query_is_stale() {
    let given = AnalysisSnapshot::new([input(1, "module Main { let 名 = 1; 名 }")]);
    let when = given.completions(FileId(2), usize::MAX);
    assert!(when.items.is_empty());
    assert!(given.definitions(FileId(2), 0).is_empty());
    assert!(given.references(FileId(2), 0, true).is_empty());
    assert!(
        given
            .inlay_hints(
                FileId(2),
                Span {
                    start: 0,
                    end: usize::MAX
                }
            )
            .is_empty()
    );
    assert!(given.completions(FileId(1), 20).items.is_empty());
}

#[test]
fn generic_method_return_stays_unknown_when_arguments_are_not_substituted() {
    let text = "module Main { fun identity<T>(value: T) -> T { value }; let result = identity(1) }";
    let given = AnalysisSnapshot::new([input(1, text)]);
    let when = given.inlay_hints(
        FileId(1),
        Span {
            start: 0,
            end: text.len(),
        },
    );
    assert!(
        !when
            .iter()
            .any(|hint| hint.offset == text.find("result").unwrap() + 6)
    );
}

#[test]
fn type_alias_navigation_when_annotation_names_alias() {
    let text = "type Text = String; module Main { let value: Text = 's' }";
    let given = AnalysisSnapshot::new([input(1, text)]);
    let when = given.definitions(FileId(1), text.rfind("Text").unwrap());
    assert_eq!(when[0].name_span.start, text.find("Text").unwrap());
}

#[test]
fn contract_requirement_navigation_when_receiver_has_contract_annotation() {
    let text = "contract Readable { public fun read() -> String } module Main { fun use(value: Readable) { value.read() } }";
    let given = AnalysisSnapshot::new([input(1, text)]);
    let when = given.definitions(FileId(1), text.rfind("read").unwrap());
    assert_eq!(when[0].name_span.start, text.find("read()").unwrap());
}

#[test]
fn no_cross_recovery_when_error_precedes_reference() {
    let text = "module Main { let value = 1; let broken = ; value }";
    let given = AnalysisSnapshot::new([input(1, text)]);
    let when = given.definitions(FileId(1), text.rfind("value").unwrap());
    assert!(when.is_empty());
}

#[test]
fn generic_construction_has_no_fabricated_type_when_arguments_are_omitted() {
    let text = "class Box<T> {} module Main { let value = Box.new() }";
    let given = AnalysisSnapshot::new([input(1, text)]);
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
fn package_segments_have_no_false_namespace_target_when_import_is_dotted() {
    let text = "module org {} import org.dep::Core";
    let given = AnalysisSnapshot::new([input(1, text)]);
    let when = given.definitions(FileId(1), text.rfind("org").unwrap());
    assert!(when.is_empty());
}

#[test]
fn closure_parameter_has_no_method_default_hint_when_context_unknown() {
    let text = "module Main { let block = { |value|; value } }";
    let given = AnalysisSnapshot::new([input(1, text)]);
    let when = given.inlay_hints(
        FileId(1),
        Span {
            start: 0,
            end: text.len(),
        },
    );
    assert!(when.is_empty());
}
