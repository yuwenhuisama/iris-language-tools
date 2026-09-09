mod completion;
mod names;
pub mod presentation;
mod receiver;

use crate::{AnalysisSnapshot, index::Key, types::TypeFact};
use iris_builtins::{BuiltinMember, BuiltinType, ReturnFact, Surface};
use iris_parser::source::{ExpressionFact, SourceKind};

#[derive(Clone, Copy)]
pub enum BuiltinReceiver {
    Instance(BuiltinType),
    Named {
        owner: &'static str,
        surface: Surface,
    },
}

impl AnalysisSnapshot {
    pub(crate) fn builtin_member(&self, key: Key, depth: usize) -> Option<&'static BuiltinMember> {
        if depth > 64 {
            return None;
        }
        let node = self.documents[&key.file].source.node(key.node);
        match &node.kind {
            SourceKind::Expression(ExpressionFact::Name { path }) if path.len() == 1 => {
                self.builtin_name_available(self.node_cursor(key), &path[0].text)
                    .then_some(())?;
                iris_builtins::members().iter().find(|member| {
                    member.surface == Surface::Global && member.selector == path[0].text
                })
            }
            SourceKind::Expression(ExpressionFact::Member {
                receiver,
                name,
                contract: false,
            }) => {
                let receiver_key = Key {
                    node: *receiver,
                    ..key
                };
                if !self.builtin_selector_available(receiver_key, &name.text) {
                    return None;
                }
                let receiver = self.builtin_receiver(receiver_key, depth + 1)?;
                let called = self.builtin_is_called(key);
                receiver
                    .members()
                    .filter(|member| {
                        member.selector == name.text
                            && self.builtin_unchanged(key.file, member)
                            && (!called || member.surface != Surface::Property)
                    })
                    .min_by_key(|member| {
                        usize::from(!called && member.surface != Surface::Property)
                    })
            }
            SourceKind::Expression(ExpressionFact::Grouped { value }) => self.builtin_member(
                Key {
                    node: *value,
                    ..key
                },
                depth + 1,
            ),
            _ => None,
        }
    }

    pub(crate) fn builtin_result(&self, key: Key, depth: usize) -> Option<TypeFact> {
        let member = self.builtin_member(key, depth + 1)?;
        let called = self.builtin_is_called(key);
        if !called
            && member.receiver == Some(BuiltinType::Object)
            && member.surface != Surface::Property
        {
            return None;
        }
        if called && member.shapes.is_empty()
            || !called
                && member.surface != Surface::Property
                && !member
                    .shapes
                    .iter()
                    .any(|shape| shape.parameters.is_empty())
        {
            return None;
        }
        let kind = match member.result {
            ReturnFact::Unknown => return None,
            ReturnFact::ArrayOf(element) => return Some(TypeFact::ArrayOf(element)),
            ReturnFact::Known(kind) => kind,
            ReturnFact::Receiver => member.receiver?,
        };
        Some(TypeFact::Builtin {
            kind,
            label: kind.name().to_owned(),
        })
    }

    fn builtin_is_called(&self, key: Key) -> bool {
        self.documents[&key.file].parents[key.node.0].is_some_and(|parent| {
            matches!(&self.documents[&key.file].source.node(parent).kind,
                SourceKind::Expression(ExpressionFact::Call {callee, ..}) if *callee == key.node)
        })
    }
}
