use crate::{
    AnalysisSnapshot, DocumentationInfo, FileId, HoverInfo, HoverKind, ParameterCategory,
    SignatureInfo, SignatureParameterInfo, Span, index::Key,
};
use iris_builtins::{Availability, BuiltinMember, CallShape, ParameterKind, Surface};
use iris_parser::source::{ExpressionFact, SourceKind};

pub fn signature(member: &BuiltinMember, shape: &CallShape) -> Option<SignatureInfo> {
    let surface = match member.surface {
        Surface::Instance | Surface::Global | Surface::Service => "",
        Surface::Class => "class ",
        Surface::Property => "property ",
    };
    let mut label = format!("builtin {surface}fun {}(", member.selector);
    let mut parameters = Vec::with_capacity(shape.parameters.len());
    for parameter in shape.parameters {
        if !parameters.is_empty() {
            label.push_str(", ");
        }
        let start = label.len();
        let (prefix, category) = match parameter.kind {
            ParameterKind::Positional => ("", ParameterCategory::Positional),
            ParameterKind::Keyword => ("key ", ParameterCategory::Keyword),
            ParameterKind::Rest => ("*", ParameterCategory::Rest),
            ParameterKind::Block => ("&", ParameterCategory::Block),
        };
        label.push_str(prefix);
        label.push_str(parameter.label);
        if let Some(kind) = parameter.type_label {
            label.push_str(": ");
            label.push_str(kind);
        }
        if parameter.optional {
            label.push_str(" [optional]");
        }
        parameters.push(SignatureParameterInfo {
            label: Span {
                start,
                end: label.len(),
            },
            name: parameter.label.into(),
            category,
            docs: None,
        });
    }
    label.push(')');
    if let Some(result) = member.return_label {
        label.push_str(" -> ");
        label.push_str(result);
    }
    if label.len() > 4096 {
        return None;
    }
    let availability = match shape.availability {
        Availability::Both => "Reference evaluator and VM",
        Availability::Reference => "Reference evaluator only",
        Availability::Vm => "VM only",
    };
    let mut text = format!(
        "{}\n\nBuiltin implementation evidence: {}\nAvailability: {availability}.",
        member.documentation, member.evidence
    );
    let truncated = text.len() > 2048;
    if truncated {
        let mut end = 2048 - "... [truncated]".len();
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
        text.push_str("... [truncated]");
    }
    Some(SignatureInfo {
        label,
        parameters,
        docs: Some(DocumentationInfo { text, truncated }),
        active_parameter: None,
    })
}

impl AnalysisSnapshot {
    pub(crate) fn builtin_hover(&self, file: FileId, byte: usize) -> Option<HoverInfo> {
        let document = &self.documents[&file];
        for node in &document.source.nodes {
            let site = match &node.kind {
                SourceKind::Expression(ExpressionFact::Member { name, .. }) => Some(name),
                SourceKind::Expression(ExpressionFact::Name { path }) => path.last(),
                _ => None,
            };
            let Some(site) = site.filter(|site| site.span.start <= byte && byte < site.span.end)
            else {
                continue;
            };
            let key = Key {
                file,
                node: node.id,
            };
            if site.text == "Transformation"
                && matches!(&node.kind, SourceKind::Expression(ExpressionFact::Name {path}) if path.len() == 1)
                && self.builtin_name_available(self.node_cursor(key), &site.text)
            {
                return Some(HoverInfo {
                    span: site.span,
                    signature: "builtin value Transformation".into(),
                    type_label: Some("Transformation".into()),
                    kind: HoverKind::Constant,
                    owner: None,
                    details: Vec::new(),
                    docs: None,
                });
            }
            if let Some(member) = self.builtin_member(key, 0) {
                let mut signatures = member
                    .shapes
                    .iter()
                    .filter_map(|shape| signature(member, shape));
                let (mut label, mut docs) = signatures.next().map_or_else(
                    || {
                        (
                            format!("builtin {}.{}", member.owner, member.selector),
                            Some(DocumentationInfo {
                                text: member.documentation.into(),
                                truncated: false,
                            }),
                        )
                    },
                    |signature| (signature.label, signature.docs),
                );
                for signature in signatures {
                    if label.len() + signature.label.len() < 3072 {
                        label.push('\n');
                        label.push_str(&signature.label);
                    }
                    if let (Some(docs), Some(alternative)) = (&mut docs, signature.docs)
                        && let Some(availability) = alternative.text.rsplit("Availability: ").next()
                        && !docs.text.contains(availability)
                        && docs.text.len() + availability.len() + 25 <= 2048
                    {
                        docs.text.push_str("\nAlternative availability: ");
                        docs.text.push_str(availability);
                    }
                }
                return Some(HoverInfo {
                    span: site.span,
                    signature: label,
                    type_label: member.return_label.map(str::to_owned),
                    kind: if member.surface == Surface::Property {
                        HoverKind::Property
                    } else {
                        HoverKind::Method
                    },
                    owner: Some(member.owner.into()),
                    details: Vec::new(),
                    docs,
                });
            }
            if let Some(super::BuiltinReceiver::Named { owner, surface }) =
                self.builtin_receiver(key, 0)
            {
                return Some(HoverInfo {
                    span: site.span,
                    signature: format!(
                        "builtin {} {owner}",
                        if surface == Surface::Class {
                            "class"
                        } else {
                            "service"
                        }
                    ),
                    type_label: None,
                    kind: if surface == Surface::Class {
                        HoverKind::Class
                    } else {
                        HoverKind::Module
                    },
                    owner: None,
                    details: Vec::new(),
                    docs: None,
                });
            }
        }
        None
    }
}
