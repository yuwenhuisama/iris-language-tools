use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput, Span};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

fn label(snapshot: &AnalysisSnapshot, text: &str, binding: &str) -> Option<String> {
    snapshot
        .hover(FileId(1), text.find(binding).unwrap())
        .and_then(|hover| hover.type_label)
}

#[test]
fn infers_nullable_result_when_source_chain_contains_safe_steps() {
    // Given known source return annotations and a nullable starting value.
    let text = "class A { public fun b() -> B {} } class B { public fun c() -> C {} } class C { public fun d() -> D {} } class D { public fun e() -> Integer {} } module Main { fun run(a: A?) { let result=a?.b()?.c()?.d()?.e(); let again=result; } }";
    let given = snapshot(text);

    // When inspecting the guarded expression and its copy.
    let result = label(&given, text, "result=");
    let again = label(&given, text, "again=");

    // Then both have one nullable label, not a pile of question marks.
    assert_eq!(result.as_deref(), Some("Integer?"));
    assert_eq!(again.as_deref(), Some("Integer?"));
}

#[test]
fn infers_array_result_when_indexed_element_is_safely_called() {
    // Given a structurally known array element with an out-of-bounds nil possibility.
    let text = "let sections='abc'.split(''); let words=sections[0]?.split(\"\\n\");";
    let given = snapshot(text);

    // When inspecting the guarded builtin call.
    let result = label(&given, text, "words=");

    // Then the element type survives the guarded call and the result stays nullable.
    assert_eq!(result.as_deref(), Some("Array<String>?"));
}

#[test]
fn infers_both_indexed_postfix_chains_in_one_source() {
    // Given two reads of the same known array in a single source.
    let text = "let sections=\"ffff\".split(\"\"); let lines=sections[0]?.split(\"\\n\"); let certain=sections[0]!.split(\"\\n\");";
    let given = snapshot(text);

    // When inspecting each chain's result.
    let lines = label(&given, text, "lines=");
    let certain = label(&given, text, "certain=");

    // Then both indexed reads retain the same known element family.
    assert_eq!(lines.as_deref(), Some("Array<String>?"));
    assert_eq!(certain.as_deref(), Some("Array<String>"));
}

#[test]
fn completes_partial_member_after_direct_index_assertion() {
    // Given a known array and an asserted indexed element.
    let text = "let sections='ffff'.split(''); sections[0]!.re";
    let given = snapshot(text);

    // When requesting a partial member completion.
    let items = given.completions(FileId(1), text.len()).items;

    // Then String members matching the prefix are offered.
    assert!(items.iter().any(|item| item.label == "replace"));
}

#[test]
fn preserves_element_after_guarded_index_when_array_identity_is_known() {
    // Given a known array returned by a source method.
    let text = "class Box { public fun parts() -> Array<String> {} } module Main { fun use(value: Box?) { let first=value?.parts()[0]; } }";
    let given = snapshot(text);

    // When inspecting an indexed guarded result.
    let result = label(&given, text, "first=");

    // Then the index retains the element label without adding another question mark.
    assert_eq!(result.as_deref(), Some("String?"));
}

#[test]
fn narrows_only_the_non_null_expression_when_asserting() {
    // Given an immutable nullable binding and a known array index result.
    let text = "let sections='abc'.split(''); let value=sections[0]; let asserted=value!; let copied=value; let words=value!.split(\"\\n\");";
    let given = snapshot(text);

    // When inspecting assertion and later uses of the binding.
    let asserted = label(&given, text, "asserted=");
    let copied = label(&given, text, "copied=");
    let words = label(&given, text, "words=");

    // Then only the expression is narrowed; the original binding remains nullable.
    assert_eq!(asserted.as_deref(), Some("String"));
    assert_eq!(copied.as_deref(), Some("String?"));
    assert_eq!(words.as_deref(), Some("Array<String>"));
}

#[test]
fn offers_members_for_guarded_or_asserted_receivers_but_not_ordinary_nullable_ones() {
    // Given a nullable String from a known array index.
    for (suffix, expected) in [("?.spl", true), ("!.spl", true), (".spl", false)] {
        let text = format!("let sections='abc'.split(''); let value=sections[0]; value{suffix}");
        let given = snapshot(&text);

        // When requesting completion at the postfix selector.
        let items = given.completions(FileId(1), text.len()).items;

        // Then only guarded or asserted access exposes String members.
        assert_eq!(
            items.iter().any(|item| item.label == "split"),
            expected,
            "{text}"
        );
    }
}

#[test]
fn retains_guarded_source_member_identity_and_result_hints() {
    // Given a guarded method on a known source Class.
    let text = "class Box { public fun read() -> String {} } module Main { fun use(value: Box?) { let guarded=value?.read(); let asserted=value!.read(); let original=value; } }";
    let given = snapshot(text);

    // When querying navigation, hover, and hints.
    let occurrence = text.find("value?.read").unwrap() + "value?.".len();
    let targets = given.definitions(FileId(1), occurrence);
    let hovered = given.hover(FileId(1), occurrence);
    let hints = given.inlay_hints(
        FileId(1),
        Span {
            start: 0,
            end: text.len(),
        },
    );

    // Then guarded navigation resolves its declaration and only guarded result is nullable.
    assert_eq!(targets.len(), 1);
    assert_eq!(
        hovered.and_then(|hover| hover.type_label).as_deref(),
        Some("String")
    );
    assert_eq!(label(&given, text, "guarded=").as_deref(), Some("String?"));
    assert_eq!(label(&given, text, "asserted=").as_deref(), Some("String"));
    assert!(hints.iter().any(|hint| hint.label == ": String?"));
}

#[test]
fn completes_source_members_only_under_guard_or_assertion() {
    // Given a nullable source Class receiver with a public member.
    for (suffix, expected) in [("?.rea", true), ("!.rea", true), (".rea", false)] {
        let text = format!(
            "class Box {{ public fun read() -> String {{}} }} module Main {{ fun use(value: Box?) {{ value{suffix}"
        );
        let given = snapshot(&text);

        // When requesting completion on its selector.
        let items = given.completions(FileId(1), text.len()).items;

        // Then ordinary access never claims a definite source member.
        assert_eq!(
            items.iter().any(|item| item.label == "read"),
            expected,
            "{text}"
        );
    }
}

#[test]
fn refuses_unguarded_nullable_member_inference_and_unknown_results() {
    // Given known nullable receivers and an unknown/dynamic expression.
    for (setup, expression) in [
        ("let parts='abc'.split('');", "parts[0].split('')"),
        ("let parts='abc'.split('');", "parts[0].split"),
        ("fun use(value: Dynamic<Object>) {", "value?.split('')"),
    ] {
        let text = if setup.starts_with("fun") {
            format!("module Main {{ {setup} let result={expression}; }} }}")
        } else {
            format!("{setup} let result={expression};")
        };
        let given = snapshot(&text);

        // When inspecting an unguarded nullable access or a dynamic guarded access.
        let result = label(&given, &text, "result=");

        // Then analysis declines to assert a definite member result.
        assert_eq!(result, None, "{text}");
    }
}

#[test]
fn carries_guard_across_ordinary_member_call_and_index_but_stops_at_grouping() {
    // Given a known source member returning an array and a nullable owner.
    let text = "class Box { public fun parts() -> Array<String> {} } module Main { fun use(value: Box?) { let first=value?.parts()[0]; let second=value?.parts().size; let outside=(value?.parts())[0]; } }";
    let given = snapshot(text);

    // When inspecting dependent postfix steps and an access outside the guarded chain.
    let first = label(&given, text, "first=");
    let outside = label(&given, text, "outside=");

    // Then guarded index retains one optional element while grouping ends the guard.
    assert_eq!(first.as_deref(), Some("String?"));
    assert_eq!(outside, None);
}

#[test]
fn refuses_guarded_element_result_after_possible_array_mutation() {
    // Given a mutable alias that invalidates known element contents.
    let text = "let parts='abc'.split(''); mut copy=parts; let value=parts[0]?.split(\"\\n\");";
    let given = snapshot(text);

    // When inspecting the guarded call on the uncertain element.
    let result = label(&given, text, "value=");

    // Then the guard does not fabricate an element type.
    assert_eq!(result, None);
}

#[test]
fn preserves_explicit_optional_builtin_and_source_annotations() {
    // Given explicitly nullable known types and a nullable source return.
    let text = "class Box { public fun next() -> Box? {} public fun value() -> String {} } module Main { fun use(item: Box?, text: String?, array: Array<String>?) { let next=item?.next()?.value(); let lines=text?.split(\"\\n\"); let length=array?.length(); let original=item; } }";
    let given = snapshot(text);

    // When inspecting guarded results and the untouched optional binding.
    let next = label(&given, text, "next=");
    let lines = label(&given, text, "lines=");
    let length = label(&given, text, "length=");
    let original = label(&given, text, "original=");

    // Then known inner facts survive optional annotations and repeated guards.
    assert_eq!(next.as_deref(), Some("String?"));
    assert_eq!(lines.as_deref(), Some("Array<String>?"));
    assert_eq!(length.as_deref(), Some("Integer?"));
    assert_eq!(original.as_deref(), Some("Box?"));
}

#[test]
fn preserves_unknown_optional_annotation_label_without_inventing_members() {
    // Given a generic optional annotation without supported element substitution.
    let text = "module Main { fun use(value: Dynamic<String>?) { let copy=value; } }";
    let given = snapshot(text);

    // When inspecting the copied binding.
    let result = label(&given, text, "copy=");

    // Then the written label survives without speculative inner inference.
    assert_eq!(result.as_deref(), Some("Dynamic<String>?"));
}

#[test]
fn refuses_inference_after_guarded_array_escapes_or_is_mutated() {
    // Given a known array whose later uses may alter its element family.
    for use_site in ["parts.push(1);", "unknown(parts);"] {
        let text =
            format!("let parts='abc'.split(''); {use_site} let value=parts[0]?.split(\"\\n\");");
        let given = snapshot(&text);

        // When inspecting the guarded use of its uncertain element.
        let result = label(&given, &text, "value=");

        // Then safe navigation does not reinstate invalidated structural facts.
        assert_eq!(result, None, "{text}");
    }
}
