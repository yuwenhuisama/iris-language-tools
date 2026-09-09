use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};
use iris_builtins::{BuiltinMember, Surface};

#[test]
fn completion_covers_catalog_when_receiver_route_is_proven() {
    for member in iris_builtins::members() {
        let text = receiver_prefix(member);
        let given = AnalysisSnapshot::new([SourceInput {
            id: FileId(1),
            group: GroupId(1),
            text: text.clone().into(),
        }]);

        let when = given.completions(FileId(1), text.len());

        assert!(
            when.items.iter().any(|item| item.label == member.selector),
            "{text}: {}",
            member.selector
        );
    }
}

#[test]
fn signature_slots_are_valid_when_catalog_signatures_are_rendered() {
    for member in iris_builtins::members() {
        if member.surface == Surface::Property {
            continue;
        }
        let prefix = receiver_prefix(member);
        let text = format!("{prefix}{}(", member.selector);
        if member.selector == "/" {
            continue;
        }
        let given = AnalysisSnapshot::new([SourceInput {
            id: FileId(1),
            group: GroupId(1),
            text: text.clone().into(),
        }]);

        let when = given.signature_help(FileId(1), text.len());

        if member.shapes.is_empty() {
            assert_eq!(when, None, "{text}");
            continue;
        }
        let help = when.unwrap_or_else(|| panic!("{text}"));
        assert_eq!(help.signatures.len(), member.shapes.len(), "{text}");
        for signature in help.signatures {
            assert!(signature.label.len() <= 4096);
            for parameter in signature.parameters {
                assert!(
                    signature
                        .label
                        .get(parameter.label.start..parameter.label.end)
                        .is_some(),
                    "{text}"
                );
            }
        }
    }
}

fn annotation(owner: &str) -> &str {
    match owner {
        "Closure" => "Closure<(Object) -> Object>",
        "BoundMethod" => "BoundMethod<(Object) -> Object>",
        _ => owner,
    }
}

fn receiver_prefix(member: &BuiltinMember) -> String {
    match member.surface {
        Surface::Global => "module Main { ".into(),
        Surface::Service => format!("module Main {{ {}.", member.owner),
        Surface::Class | Surface::Property if member.owner == "Module" => {
            "module Target {} module Main { Target.".into()
        }
        Surface::Class => match member.owner {
            "Class" => "module Main { fun use(value: Class) { value.".into(),
            owner => format!("module Main {{ {owner}."),
        },
        Surface::Property
            if member.receiver.is_none()
                || iris_builtins::members().iter().any(|candidate| {
                    candidate.owner == member.owner
                        && candidate.selector == member.selector
                        && candidate.surface == Surface::Class
                }) && iris_builtins::class_names().contains(&member.owner) =>
        {
            format!("module Main {{ {}.", member.owner)
        }
        Surface::Instance | Surface::Property => format!(
            "module Main {{ fun use(value: {}) {{ value.",
            annotation(member.owner)
        ),
    }
}
