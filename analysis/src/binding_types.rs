use crate::{AnalysisSnapshot, index::Key, types::TypeFact};
use iris_parser::source::{DeclarationKind, ScopeKind};
use iris_syntax::ParameterCategory;

#[cfg(test)]
mod tests;

impl AnalysisSnapshot {
    pub(crate) fn binding_type(&self, key: Key, depth: usize) -> Option<TypeFact> {
        if depth > 64 {
            return None;
        }
        let symbol = self.symbol(key)?;
        if self.declaration_key(symbol) != Some(key) {
            return None;
        }
        match symbol.declaration.kind {
            DeclarationKind::Class | DeclarationKind::Module | DeclarationKind::Contract => {
                Some(TypeFact::Object(key))
            }
            DeclarationKind::Parameter => {
                let element = symbol.declaration.annotation.map_or_else(
                    || {
                        (self.documents[&key.file].source.scope(symbol.scope).kind
                            == ScopeKind::Method)
                            .then_some(TypeFact::Literal("Dynamic<Object>"))
                    },
                    |node| {
                        self.annotation_type(Key {
                            file: key.file,
                            node,
                        })
                    },
                )?;
                match symbol.declaration.parameter_category {
                    Some(ParameterCategory::Rest) => Some(TypeFact::Builtin {
                        kind: iris_builtins::BuiltinType::from_name("Array")?,
                        label: format!("Array<{}>", self.type_label(element)?),
                    }),
                    Some(ParameterCategory::KeywordRest) => Some(TypeFact::Builtin {
                        kind: iris_builtins::BuiltinType::from_name("Hash")?,
                        label: format!("Hash<Symbol, {}>", self.type_label(element)?),
                    }),
                    Some(
                        ParameterCategory::Positional
                        | ParameterCategory::Keyword
                        | ParameterCategory::Block,
                    )
                    | None => Some(element),
                }
            }
            DeclarationKind::Binding
            | DeclarationKind::Constant
            | DeclarationKind::TypeAlias
            | DeclarationKind::Global
            | DeclarationKind::Shared
            | DeclarationKind::Property
            | DeclarationKind::Method
            | DeclarationKind::TypeParameter
            | DeclarationKind::PatternBinding => {
                if let Some(node) = symbol.declaration.annotation {
                    return self.annotation_type(Key {
                        file: key.file,
                        node,
                    });
                }
                if symbol.declaration.modifiers.mutable {
                    return None;
                }
                match symbol.declaration.kind {
                    DeclarationKind::Binding | DeclarationKind::Constant => self.expression_type(
                        Key {
                            file: key.file,
                            node: symbol.declaration.initializer?,
                        },
                        depth + 1,
                    ),
                    DeclarationKind::Class
                    | DeclarationKind::Module
                    | DeclarationKind::Contract
                    | DeclarationKind::Parameter
                    | DeclarationKind::TypeAlias
                    | DeclarationKind::Global
                    | DeclarationKind::Shared
                    | DeclarationKind::Property
                    | DeclarationKind::Method
                    | DeclarationKind::TypeParameter
                    | DeclarationKind::PatternBinding => None,
                }
            }
        }
    }
}
