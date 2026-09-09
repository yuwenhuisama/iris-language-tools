use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};

fn snapshot(text: &str) -> AnalysisSnapshot {
    AnalysisSnapshot::new([SourceInput {
        id: FileId(1),
        group: GroupId(1),
        text: text.into(),
    }])
}

#[test]
fn empty_signature_when_editor_has_only_open_parenthesis() {
    let text = "module Main { fun read() {} read(";
    let given = snapshot(text);
    let when = given.signature_help(FileId(1), text.len()).unwrap();
    assert_eq!(when.signatures[0].active_parameter, None);
}

#[test]
fn signature_is_absent_when_keyword_and_block_parameters_precede_positionals() {
    let text = "module Main { fun read(key option, &block, value, *rest) {} read(1, 2";
    let given = snapshot(text);
    let when = given.signature_help(FileId(1), text.len());
    assert_eq!(when, None);
}

#[test]
fn positional_mapping_when_parameters_follow_channel_order() {
    let text = "module Main { fun read(value, *rest, key option, &block) {} read(1, 2";
    let given = snapshot(text);
    let when = given.signature_help(FileId(1), text.len()).unwrap();
    assert_eq!(when.signatures[0].active_parameter, Some(1));
}

#[test]
fn hole_mapping_when_cursor_is_in_trivia_before_boundary() {
    for suffix in ["read(1,   ) }", "read(1,   }", "read(1,   "] {
        let text = format!("module Main {{ fun read(first, second) {{}} {suffix}");
        let given = snapshot(&text);
        let when = given
            .signature_help(FileId(1), text.rfind(',').unwrap() + 2)
            .unwrap();
        assert_eq!(when.signatures[0].active_parameter, Some(1), "{suffix}");
    }
}

#[test]
fn signature_is_absent_when_recovery_precedes_callee() {
    let text = "module Main { fun read(value) {} let broken = ; read(";
    let given = snapshot(text);
    let when = given.signature_help(FileId(1), text.len());
    assert_eq!(when, None);
}

#[test]
fn signature_is_absent_when_call_uses_unsupported_splat() {
    for call in ["read(*items", "read(**items", "unknown(", "block("] {
        let text =
            format!("module Main {{ fun read(value) {{}} let block = {{ |value|; value }}; {call}");
        let given = snapshot(&text);
        let when = given.signature_help(FileId(1), text.len());
        assert_eq!(when, None, "{call}");
    }
}

#[test]
fn signature_is_absent_when_cursor_is_not_a_utf8_boundary_or_outside_call() {
    let text = "module Main { fun read(value) {} read('😀') }";
    let given = snapshot(text);
    for byte in [
        text.find('😀').unwrap() + 1,
        text.rfind("read").unwrap() + 4,
        text.rfind(')').unwrap() + 1,
        text.len() + 1,
        usize::MAX,
    ] {
        let when = given.signature_help(FileId(1), byte);
        assert_eq!(when, None, "{byte}");
    }
    assert_eq!(given.signature_help(FileId(9), 0), None);
}

#[test]
fn inner_complete_call_when_cursor_precedes_inner_closer() {
    let text = "module Main { fun outer(first,second) {} fun inner(value) {} outer(1, inner(2)) }";
    let given = snapshot(text);
    let when = given
        .signature_help(FileId(1), text.rfind("2)").unwrap() + 1)
        .unwrap();
    assert!(when.signatures[0].label.contains("inner("));
    assert_eq!(when.signatures[0].active_parameter, Some(0));
}

#[test]
fn editor_keyword_when_reserved_name_is_retained_as_damaged_slot() {
    let text = "module Main { fun read(**options) {} read(key:";
    let given = snapshot(text);
    let when = given.signature_help(FileId(1), text.len()).unwrap();
    assert_eq!(when.signatures[0].active_parameter, Some(0));
}

#[test]
fn nested_partial_expression_when_only_inner_argument_is_damaged() {
    let text =
        "module Main { fun outer(first,second) {} fun inner(value) {} outer(1, inner(2 + )) }";
    let given = snapshot(text);
    let when = given
        .signature_help(FileId(1), text.rfind("))").unwrap())
        .unwrap();
    assert!(when.signatures[0].label.contains("inner("));
    assert_eq!(when.signatures[0].active_parameter, Some(0));
}

#[test]
fn prefix_queries_when_source_contains_astral_defaults_are_safe() {
    let text = "module Main { fun read(value = '😀', key option) {} read(option: read(1, )) }";
    for end in (0..=text.len()).filter(|end| text.is_char_boundary(*end)) {
        let given = snapshot(&text[..end]);
        for byte in 0..=end {
            let when = given.signature_help(FileId(1), byte);
            if let Some(help) = when {
                let signature = &help.signatures[help.active_signature];
                assert!(text.is_char_boundary(byte));
                assert!(signature.label.len() <= 4096);
                for parameter in &signature.parameters {
                    assert!(
                        signature
                            .label
                            .get(parameter.label.start..parameter.label.end)
                            .is_some()
                    );
                }
            }
        }
    }
}

#[test]
fn trailing_block_is_not_active_when_cursor_is_inside_parentheses() {
    let text = "module Main { fun read(&block) {} read() { ||; 1 } }";
    let given = snapshot(text);
    let when = given.signature_help(FileId(1), text.rfind("read(").unwrap() + 5);
    assert_eq!(when, None);
}

#[test]
fn method_queries_are_absent_when_header_errors_follow_closing_parenthesis() {
    for header in [
        "read(value, key option, second)",
        "read(value, key option, second) -> String",
        "read(value, *first, *second)",
        "read(value, **first, **second)",
        "read(value, &first, &second)",
        "read<>(value, second)",
        "read(value = , second)",
    ] {
        for body in [" {} ", "\n{}\n", "\n"] {
            let text = format!("module Main {{ fun {header}{body}read(1,");
            let given = snapshot(&text);
            let when = (
                given.hover(FileId(1), text.find("read").unwrap()),
                given.hover(FileId(1), text.rfind("read").unwrap()),
                given.signature_help(FileId(1), text.len()),
            );
            assert_eq!(when, (None, None, None), "{text}");
        }
    }
}

#[test]
fn method_queries_remain_usable_when_only_body_or_call_is_incomplete() {
    for text in [
        "module Main { fun read(value, second) {} read(1,",
        "module Main { fun read(value, second) { read(1,",
        "module Main { fun read(_, _) { read(1,",
        "module Main { fun read(_, _, *rest, key option, **kwargs, &block) {} read(1,",
    ] {
        let given = snapshot(text);
        let when = (
            given.hover(FileId(1), text.find("read").unwrap()),
            given.hover(FileId(1), text.rfind("read").unwrap()),
            given.signature_help(FileId(1), text.len()),
        );
        assert!(when.0.is_some(), "declaration: {text}");
        assert!(when.1.is_some(), "call: {text}");
        assert_eq!(
            when.2.unwrap().signatures[0].active_parameter,
            Some(1),
            "{text}"
        );
    }
}
