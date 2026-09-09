use super::text::BoundedText;
use crate::{AnalysisSnapshot, index::Key, types::TypeFact};
use iris_parser::source::{DeclarationKind, ExpressionFact, ScopeKind, SourceKind, SyntaxId};
use iris_syntax::{ParameterCategory, TypeExpression};

impl AnalysisSnapshot {
    pub(super) fn hover_label(&self, key: Key, depth: usize) -> Option<String> {
        if depth > 64 {
            return None;
        }
        let symbol = self.symbol(key)?;
        if self.declaration_key(symbol) != Some(key) {
            return None;
        }
        let declaration = &symbol.declaration;
        match declaration.kind {
            DeclarationKind::Class | DeclarationKind::Module | DeclarationKind::Contract => None,
            DeclarationKind::Method => declaration.return_type.map_or_else(
                || Some("Dynamic<Object>".into()),
                |node| self.hover_annotation(Key { node, ..key }),
            ),
            DeclarationKind::Parameter => {
                let element = declaration.annotation.map_or_else(
                    || {
                        (self.documents[&key.file].source.scope(symbol.scope).kind
                            == ScopeKind::Method)
                            .then(|| "Dynamic<Object>".into())
                    },
                    |node| self.hover_annotation(Key { node, ..key }),
                )?;
                let mut label = BoundedText::new(1024);
                let (prefix, suffix) = match declaration.parameter_category {
                    Some(ParameterCategory::Rest) => ("Array<", ">"),
                    Some(ParameterCategory::KeywordRest) => ("Hash<Symbol, ", ">"),
                    Some(
                        ParameterCategory::Positional
                        | ParameterCategory::Keyword
                        | ParameterCategory::Block,
                    )
                    | None => ("", ""),
                };
                label.push(prefix);
                label.push(&element);
                label.push(suffix);
                Some(label.finish())
            }
            DeclarationKind::Binding
            | DeclarationKind::Constant
            | DeclarationKind::TypeAlias
            | DeclarationKind::Global
            | DeclarationKind::Shared
            | DeclarationKind::Property
            | DeclarationKind::TypeParameter
            | DeclarationKind::PatternBinding => {
                if let Some(node) = declaration.annotation {
                    return self.hover_annotation(Key { node, ..key });
                }
                match declaration.kind {
                    DeclarationKind::Binding | DeclarationKind::Constant => self.hover_expression(
                        Key {
                            node: declaration.initializer?,
                            ..key
                        },
                        depth + 1,
                    ),
                    _ => None,
                }
            }
        }
    }

    fn hover_annotation(&self, key: Key) -> Option<String> {
        let document = &self.documents[&key.file];
        let node = document.source.node(key.node);
        match &node.kind {
            SourceKind::Type(TypeExpression::Typeof(_)) => None,
            SourceKind::Type(_) => {
                let mut label = BoundedText::new(1024);
                document.hover_source(node.span, &mut label);
                Some(label.finish())
            }
            _ => None,
        }
    }

    fn hover_expression(&self, key: Key, depth: usize) -> Option<String> {
        if depth > 64 {
            return None;
        }
        let node = self.documents[&key.file].source.node(key.node);
        let SourceKind::Expression(expression) = &node.kind else {
            return None;
        };
        match expression {
            ExpressionFact::Name { path } => {
                self.hover_label(self.path(self.node_cursor(key), path)?, depth + 1)
            }
            ExpressionFact::Member { .. } => {
                let target = self.expression_symbol(key, depth + 1)?;
                if self.symbol(target)?.declaration.kind == DeclarationKind::Method {
                    return None;
                }
                self.hover_label(target, depth + 1)
            }
            ExpressionFact::Grouped { value } => self.hover_expression(
                Key {
                    node: *value,
                    ..key
                },
                depth + 1,
            ),
            ExpressionFact::Call {
                callee,
                type_arguments,
                ..
            } => {
                if !type_arguments.is_empty() {
                    return None;
                }
                self.hover_call(
                    Key {
                        node: *callee,
                        ..key
                    },
                    depth,
                )
            }
            ExpressionFact::Literal { .. }
            | ExpressionFact::Array { .. }
            | ExpressionFact::Tuple { .. }
            | ExpressionFact::Hash { .. }
            | ExpressionFact::Range { .. } => self.type_label(self.expression_type(key, depth)?),
            ExpressionFact::IncompleteMember { .. }
            | ExpressionFact::Assignment { .. }
            | ExpressionFact::Construction { .. }
            | ExpressionFact::ReifiedType { .. }
            | ExpressionFact::Closure { .. }
            | ExpressionFact::KeywordArgument { .. }
            | ExpressionFact::Unsupported { .. } => None,
        }
    }

    fn hover_call(&self, callee: Key, depth: usize) -> Option<String> {
        if let Some(method) = self.expression_symbol(callee, depth + 1) {
            let symbol = self.symbol(method)?;
            let document = &self.documents[&method.file];
            if symbol.declaration.kind != DeclarationKind::Method
                || symbol.declaration.modifiers.asynchronous
                || document.source.node(method.node).children.iter().any(|child| matches!(
                    &document.source.node(*child).kind,
                    SourceKind::Declaration(value) if value.kind == DeclarationKind::TypeParameter
                ))
            {
                return None;
            }
            return self.hover_label(method, depth + 1);
        }
        let parent: SyntaxId = self.documents[&callee.file].parents[callee.node.0]?;
        let fact = self.expression_type(
            Key {
                node: parent,
                ..callee
            },
            depth,
        )?;
        let owner = match fact {
            TypeFact::Instance(owner) => owner,
            TypeFact::Builtin { label, .. } => return Some(label),
            TypeFact::Literal(_)
            | TypeFact::Written { .. }
            | TypeFact::Object(_)
            | TypeFact::BuiltinClass(_) => return None,
        };
        let mut label = BoundedText::new(1024);
        for (index, segment) in self.symbol(owner)?.qualified.as_ref()?.iter().enumerate() {
            if index > 0 {
                label.push("::");
            }
            label.push(segment);
            if label.full() {
                break;
            }
        }
        Some(label.finish())
    }
}
