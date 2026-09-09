use crate::{AnalysisSnapshot, index::Key, types::TypeFact};
use iris_builtins::BuiltinType;
use iris_parser::source::{DeclarationKind, ExpressionFact, SourceKind, SyntaxId};
use iris_syntax::TypeExpression;

impl AnalysisSnapshot {
    pub(crate) fn array_annotation_element(
        &self,
        key: Key,
        annotation: &TypeExpression,
    ) -> Option<BuiltinType> {
        let TypeExpression::Generic { name, arguments } = annotation else {
            return None;
        };
        let [TypeExpression::Name(element)] = arguments.as_slice() else {
            return None;
        };
        let cursor = self.node_cursor(key);
        (name == "Array"
            && self.builtin_name_available(cursor, name)
            && self.builtin_name_available(cursor, element))
        .then(|| BuiltinType::from_name(element))?
    }

    pub(crate) fn array_index_type(
        &self,
        key: Key,
        children: (SyntaxId, SyntaxId),
        depth: usize,
    ) -> Option<TypeFact> {
        let receiver = Key {
            node: children.0,
            ..key
        };
        let TypeFact::ArrayOf(element) = self.expression_type(receiver, depth)? else {
            return None;
        };
        let array_member = iris_builtins::members()
            .iter()
            .find(|member| member.receiver == Some(BuiltinType::Array))?;
        if !self.builtin_unchanged(key.file, array_member)
            || !self.array_uses_preserve_elements(receiver)
        {
            return None;
        }
        match self.expression_type(
            Key {
                node: children.1,
                ..key
            },
            depth,
        )? {
            TypeFact::Literal("Integer")
            | TypeFact::Builtin {
                kind: BuiltinType::Integer,
                ..
            } => Some(TypeFact::Nullable(element)),
            TypeFact::Literal("Range")
            | TypeFact::Builtin {
                kind: BuiltinType::Range,
                ..
            } => Some(TypeFact::ArrayOf(element)),
            _ => None,
        }
    }

    fn array_origin(&self, key: Key, depth: usize) -> Option<Key> {
        if depth > 64 {
            return None;
        }
        match &self.documents[&key.file].source.node(key.node).kind {
            SourceKind::Expression(ExpressionFact::Grouped { value }) => self.array_origin(
                Key {
                    node: *value,
                    ..key
                },
                depth + 1,
            ),
            SourceKind::Expression(ExpressionFact::Name { path }) => {
                let binding = self.path(self.node_cursor(key), path)?;
                let symbol = self.symbol(binding)?;
                symbol
                    .declaration
                    .initializer
                    .map_or(Some(binding), |node| {
                        self.array_origin(Key { node, ..binding }, depth + 1)
                    })
            }
            _ => Some(key),
        }
    }

    fn array_uses_preserve_elements(&self, receiver: Key) -> bool {
        let Some(origin) = self.array_origin(receiver, 0) else {
            return false;
        };
        let document = &self.documents[&origin.file];
        if origin.file != receiver.file || document.source.nodes.len() > 4096 {
            return false;
        }
        // Constants can expose this array to files outside the bounded local scan.
        if self.symbols.iter().any(|symbol| {
            symbol.key.file == origin.file
                && symbol.declaration.kind == DeclarationKind::Constant
                && symbol.declaration.initializer.is_some_and(|node| {
                    self.array_origin(Key { node, ..symbol.key }, 0) == Some(origin)
                })
        }) {
            return false;
        }
        document.source.nodes.iter().all(|node| {
            if !matches!(
                node.kind,
                SourceKind::Expression(ExpressionFact::Name { .. })
            ) {
                return true;
            }
            let key = Key {
                node: node.id,
                file: origin.file,
            };
            if self.array_origin(key, 0) != Some(origin) {
                return true;
            }
            if node.scope != document.source.node(origin.node).scope {
                return false;
            }
            let mut child = node.id;
            for _ in 0..64 {
                let Some(parent) = document.parents[child.0] else {
                    return false;
                };
                match &document.source.node(parent).kind {
                    SourceKind::Expression(ExpressionFact::Grouped { .. }) => child = parent,
                    SourceKind::Declaration(declaration) => {
                        return declaration.kind == DeclarationKind::Binding
                            && !declaration.modifiers.mutable
                            && document.source.node(parent).scope == node.scope;
                    }
                    SourceKind::Expression(ExpressionFact::Index { receiver, .. })
                        if *receiver == child =>
                    {
                        child = parent;
                    }
                    SourceKind::Expression(ExpressionFact::IncompleteMember { .. }) => return true,
                    _ => return false,
                }
            }
            false
        })
    }
}
