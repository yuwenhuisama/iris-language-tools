use iris_analysis::{AnalysisSnapshot, CompletionKind, FileId, GroupId, SourceInput};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn completes_metadata_when_receiver_category_survives_an_immutable_copy() {
    for (text, selector) in [
        (
            "module Main { fun use(value: Class) { value.",
            "define_method",
        ),
        (
            "module Main { fun use(value: Class) { let copy = value; copy.",
            "same?",
        ),
        (
            "module Main { fun use(value: Class) { let copy = value; copy.",
            "define_method",
        ),
        (
            "class Box {} module Main { let value = Box; let copy = (value); copy.",
            "define_method",
        ),
        (
            "module Main { let value = Float64; let copy = (value); copy.",
            "from_bits",
        ),
        ("module Main { let value = Float64; value.", "define_method"),
        (
            "contract C {} module Main { let value = C; value.",
            "parents",
        ),
        ("module M {} module Main { M.", "modules"),
    ] {
        let given = snapshot(text);

        let when = given.completions(FileId(1), text.len());

        assert!(
            when.items.iter().any(|item| item.label == selector),
            "{text}: {when:?}"
        );
    }
}

#[test]
fn returns_metadata_signatures_when_receiver_is_a_class_or_contract_value() {
    for (text, arity) in [
        (
            "module Main { fun use(value: Class) { value.define_method(",
            2,
        ),
        (
            "class Box {} module Main { let value = Box; value.define_method(",
            2,
        ),
        ("module Main { let value = Float64; value.from_bits(", 1),
        ("contract C {} module Main { C.parents(", 0),
        (
            "contract C {} module Main { let value = C; let copy = value; copy.parents(",
            0,
        ),
    ] {
        let given = snapshot(text);

        let when = given.signature_help(FileId(1), text.len());

        assert_eq!(
            when.unwrap_or_else(|| panic!("{text}")).signatures[0]
                .parameters
                .len(),
            arity
        );
    }
}

#[test]
fn completes_property_kind_when_module_metadata_is_read() {
    let text = "module M {} module Main { M.mod";
    let given = snapshot(text);

    let when = given.completions(FileId(1), text.len());

    assert!(
        when.items
            .iter()
            .any(|item| item.label == "modules" && item.kind == CompletionKind::Property)
    );
}

#[test]
fn refuses_module_metadata_when_source_route_is_copied_as_a_value() {
    let text = "module M {} module Main { let value = M; value.mod";
    let given = snapshot(text);

    let when = given.completions(FileId(1), text.len());

    assert!(!when.items.iter().any(|item| item.label == "modules"));
}

#[test]
fn refuses_metadata_when_receiver_identity_is_uncertain_or_has_a_different_category() {
    for text in [
        "module Main { let value = JSON; value.encode(",
        "module Main { let value = Encoding::UTF_8; value.decode(",
        "module Main { mut value = Float64; value.from_bits(",
        "module Main { let value = 1.0; value.from_bits(",
        "contract C {} module Main { fun use(value: C) { value.parents(",
        "class Class {} module Main { fun use(value: Class) { value.define_method(",
        "class Box {} class Box {} module Main { let value = Box; value.define_method(",
        "class Box extends Base {} module Main { let value = Box; value.define_method(",
        "class Box {} open class Box {} module Main { let value = Box; value.define_method(",
        "class Box { private class fun define_method(own) {} } module Main { let value = Box; value.define_method(",
        "class Box { public class fun define_method(first) {} public class fun define_method(second) {} } module Main { let value = Box; value.define_method(",
        "module Main { let Float64 = 1; let value = Float64; value.from_bits(",
        "open class Float64 {} module Main { let value = Float64; value.from_bits(",
        "module M {} module Main { M.modules(",
    ] {
        let given = snapshot(text);

        let when = given.signature_help(FileId(1), text.len());

        assert_eq!(when, None, "{text}");
    }
}

#[test]
fn refuses_metadata_when_source_inventory_is_partial() {
    for text in [
        "class Box {} module Main { let value = Box; value.define_method(",
        "module Main { let value = Float64; value.from_bits(",
        "contract C {} module Main { let value = C; value.parents(",
    ] {
        let given = snapshot(text).with_incomplete_groups([GroupId(1)]);

        let when = given.signature_help(FileId(1), text.len());

        assert_eq!(when, None, "{text}");
    }
}

#[test]
fn refuses_result_chains_when_heap_object_method_is_read_without_calling() {
    for text in [
        "module Main { Object.new().hash.div(",
        "module Main { Object.new().to_string.replace(",
        "module Main { let value = Object.new(); let method = value.hash; method.div(",
        "module Main { fun use(value: Object) { value.to_string.replace(",
    ] {
        let given = snapshot(text);

        let when = given.signature_help(FileId(1), text.len());

        assert_eq!(when, None, "{text}");
    }
}

#[test]
fn preserves_result_chains_when_heap_methods_are_called_or_value_properties_are_read() {
    for text in [
        "module Main { Object.new().hash().div(",
        "module Main { Object.new().to_string().replace(",
        "module Main { let value = Object; value.new().hash().div(",
        "module Main { let value = Float64; value.nan.to_bits(",
        "module Main { 'abc'.hash.div(",
        "module Main { fun use(value: Class) { value.modules.push(",
        "contract C {} module Main { let value = C; value.parents.push(",
        "module M {} module Main { M.modules.push(",
    ] {
        let given = snapshot(text);

        let when = given.signature_help(FileId(1), text.len());

        assert!(when.is_some(), "{text}");
    }
}
