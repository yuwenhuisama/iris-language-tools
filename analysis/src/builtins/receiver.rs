use super::BuiltinReceiver;
use crate::{AnalysisSnapshot, index::Key, types::TypeFact};
use iris_builtins::{BuiltinMember, BuiltinType, Surface};
use iris_parser::source::{DeclarationKind, ExpressionFact, NameSite, SourceKind};

impl BuiltinReceiver {
    pub(crate) fn members(self) -> impl Iterator<Item = &'static BuiltinMember> {
        iris_builtins::members()
            .iter()
            .filter(move |member| match self {
                Self::Instance(BuiltinType::Class) => {
                    member.receiver == Some(BuiltinType::Class)
                        && matches!(
                            member.surface,
                            Surface::Instance | Surface::Class | Surface::Property
                        )
                }
                Self::Instance(kind) => {
                    member.receiver == Some(kind)
                        && matches!(member.surface, Surface::Instance | Surface::Property)
                        && !(member.surface == Surface::Property
                            && iris_builtins::members().iter().any(|candidate| {
                                candidate.receiver == Some(kind)
                                    && candidate.selector == member.selector
                                    && candidate.surface == Surface::Class
                            }))
                }
                Self::Named { owner, surface } => {
                    let class_metadata = surface == Surface::Class && owner != "Module";
                    (member.owner == owner
                        || class_metadata
                            && member.owner == "Class"
                            && !iris_builtins::members().iter().any(|specific| {
                                specific.owner == owner
                                    && specific.surface == member.surface
                                    && specific.selector == member.selector
                            }))
                        && (member.surface == surface
                            || member.surface == Surface::Property
                                && (member.receiver.is_none()
                                    || owner == "Module"
                                    || class_metadata
                                        && iris_builtins::members().iter().any(|candidate| {
                                            candidate.owner == member.owner
                                                && candidate.selector == member.selector
                                                && candidate.surface == Surface::Class
                                        })))
                }
            })
    }
}

impl AnalysisSnapshot {
    pub(crate) fn builtin_value_type(&self, key: Key, path: &[NameSite]) -> Option<TypeFact> {
        self.builtin_name_available(self.node_cursor(key), &path.first()?.text)
            .then_some(())?;
        let name = path
            .iter()
            .map(|site| site.text.as_str())
            .collect::<Vec<_>>()
            .join("::");
        if name == "Transformation" {
            return Some(TypeFact::Literal("Transformation"));
        }
        iris_builtins::class_names()
            .iter()
            .find(|owner| **owner == name)
            .map(|owner| TypeFact::BuiltinClass(owner))
    }

    pub(crate) fn builtin_receiver(&self, key: Key, depth: usize) -> Option<BuiltinReceiver> {
        if depth > 64 {
            return None;
        }
        let node = self.documents[&key.file].source.node(key.node);
        if let SourceKind::Expression(ExpressionFact::Closure { .. }) = &node.kind {
            return Some(BuiltinReceiver::Instance(BuiltinType::Closure));
        }
        if let SourceKind::Expression(ExpressionFact::Grouped { value }) = &node.kind {
            return self.builtin_receiver(
                Key {
                    node: *value,
                    ..key
                },
                depth + 1,
            );
        }
        if let SourceKind::Expression(ExpressionFact::Name { path }) = &node.kind
            && let Some(binding) = self.path(self.node_cursor(key), path)
            && let Some(symbol) = self.symbol(binding)
            && self.declaration_key(symbol) == Some(binding)
            && symbol.declaration.annotation.is_none()
            && !symbol.declaration.modifiers.mutable
            && let Some(initializer) = symbol.declaration.initializer
            && matches!(
                &self.documents[&binding.file].source.node(initializer).kind,
                SourceKind::Expression(ExpressionFact::Closure { .. })
            )
        {
            return Some(BuiltinReceiver::Instance(BuiltinType::Closure));
        }
        if let SourceKind::Expression(ExpressionFact::Name { path }) = &node.kind
            && self.builtin_name_available(self.node_cursor(key), &path.first()?.text)
        {
            let name = path
                .iter()
                .map(|site| site.text.as_str())
                .collect::<Vec<_>>()
                .join("::");
            if let Some(owner) = iris_builtins::service_names()
                .iter()
                .find(|owner| **owner == name)
            {
                return Some(BuiltinReceiver::Named {
                    owner,
                    surface: Surface::Service,
                });
            }
        }
        match self.expression_type(key, depth + 1)? {
            TypeFact::Literal(name) => BuiltinType::from_name(name).map(BuiltinReceiver::Instance),
            TypeFact::Builtin { kind, .. } => Some(BuiltinReceiver::Instance(kind)),
            TypeFact::BuiltinClass(owner) => Some(BuiltinReceiver::Named {
                owner,
                surface: Surface::Class,
            }),
            TypeFact::Object(owner) => {
                if self.symbol(owner)?.declaration.kind == DeclarationKind::Module
                    && self.expression_symbol(key, depth + 1) != Some(owner)
                {
                    return None;
                }
                self.nominal_builtin_receiver(owner)
            }
            TypeFact::Written { .. } | TypeFact::Instance(_) => None,
        }
    }

    fn nominal_builtin_receiver(&self, owner: Key) -> Option<BuiltinReceiver> {
        let symbol = self.symbol(owner)?;
        if self.declaration_key(symbol) != Some(owner)
            || symbol.declaration.header.as_ref().is_none_or(|header| {
                !header.complete
                    || header.has_extends
                    || header.has_mixins
                    || header.has_implements
                    || header.has_decorators
                    || header.has_type_parameters
                    || header.has_constraints
            })
        {
            return None;
        }
        match symbol.declaration.kind {
            DeclarationKind::Class => Some(BuiltinReceiver::Named {
                owner: "Class",
                surface: Surface::Class,
            }),
            DeclarationKind::Module => Some(BuiltinReceiver::Named {
                owner: "Module",
                surface: Surface::Class,
            }),
            DeclarationKind::Contract => Some(BuiltinReceiver::Instance(BuiltinType::Contract)),
            DeclarationKind::TypeAlias
            | DeclarationKind::Binding
            | DeclarationKind::Constant
            | DeclarationKind::Global
            | DeclarationKind::Shared
            | DeclarationKind::Property
            | DeclarationKind::Method
            | DeclarationKind::Parameter
            | DeclarationKind::TypeParameter
            | DeclarationKind::PatternBinding => None,
        }
    }
}
