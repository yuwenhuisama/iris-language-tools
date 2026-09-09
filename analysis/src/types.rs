use crate::{AnalysisSnapshot, index::Key};
use iris_parser::source::{DeclarationKind, ExpressionFact, LiteralKind, SourceKind};
use iris_syntax::TypeExpression;

#[derive(Clone, Debug)]
pub enum TypeFact {
    BuiltinClass(&'static str),
    Builtin {
        kind: iris_builtins::BuiltinType,
        label: String,
    },
    Literal(&'static str),
    Written {
        label: String,
    },
    Instance(Key),
    Object(Key),
}

impl AnalysisSnapshot {
    pub(crate) fn annotation_type(&self, key: Key) -> Option<TypeFact> {
        let document = &self.documents[&key.file];
        let node = document.source.node(key.node);
        let SourceKind::Type(annotation) = &node.kind else {
            return None;
        };
        match annotation {
            TypeExpression::Name(_) => {
                let names: Vec<_> = node
                    .children
                    .iter()
                    .filter_map(|child| match &document.source.node(*child).kind {
                        SourceKind::Name(site) => Some(site.clone()),
                        _ => None,
                    })
                    .collect();
                if let Some(owner) = self.path(self.node_cursor(key), &names) {
                    match self.symbol(owner)?.declaration.kind {
                        DeclarationKind::Class | DeclarationKind::Contract => {
                            return Some(TypeFact::Instance(owner));
                        }
                        DeclarationKind::Module
                        | DeclarationKind::TypeAlias
                        | DeclarationKind::Binding
                        | DeclarationKind::Constant
                        | DeclarationKind::Global
                        | DeclarationKind::Shared
                        | DeclarationKind::Property
                        | DeclarationKind::Method
                        | DeclarationKind::Parameter
                        | DeclarationKind::TypeParameter
                        | DeclarationKind::PatternBinding => {}
                    }
                }
            }
            TypeExpression::Typeof(_) => return None,
            TypeExpression::Generic { .. }
            | TypeExpression::Intersection(_)
            | TypeExpression::Union(_)
            | TypeExpression::Function { .. } => {}
        }
        let label = document.input.text[node.span.start..node.span.end]
            .trim()
            .to_owned();
        let name = match annotation {
            TypeExpression::Name(name) | TypeExpression::Generic { name, .. } => Some(name),
            TypeExpression::Typeof(_)
            | TypeExpression::Intersection(_)
            | TypeExpression::Union(_)
            | TypeExpression::Function { .. } => None,
        };
        if let Some(name) = name
            && self.builtin_name_available(self.node_cursor(key), name.split("::").next()?)
            && let Some(kind) = iris_builtins::BuiltinType::from_name(name)
        {
            return Some(TypeFact::Builtin { kind, label });
        }
        Some(TypeFact::Written { label })
    }

    pub(crate) fn expression_type(&self, key: Key, depth: usize) -> Option<TypeFact> {
        if depth > 64 {
            return None;
        }
        let node = self.documents[&key.file].source.node(key.node);
        let SourceKind::Expression(expression) = &node.kind else {
            return None;
        };
        match expression {
            ExpressionFact::Array { .. } => Some(TypeFact::Literal("Array")),
            ExpressionFact::Tuple { .. } => Some(TypeFact::Literal("Tuple")),
            ExpressionFact::Hash { .. } => Some(TypeFact::Literal("Hash")),
            ExpressionFact::Range { .. } => Some(TypeFact::Literal("Range")),
            ExpressionFact::Literal { text, kind } => Some(TypeFact::Literal(match kind {
                LiteralKind::Integer => "Integer",
                LiteralKind::Float => {
                    if text.ends_with("f32") {
                        "Float32"
                    } else {
                        "Float64"
                    }
                }
                LiteralKind::String => "String",
                LiteralKind::MutableString => "MutableString",
                LiteralKind::Bytes => "Bytes",
                LiteralKind::ByteArray => "ByteArray",
                LiteralKind::Regex => "Regex",
                LiteralKind::Symbol => "Symbol",
                LiteralKind::Bool => "Bool",
                LiteralKind::Nil => "Nil",
            })),
            ExpressionFact::Name { path } => {
                if path.len() == 1 && path[0].text == "self" {
                    let receiver = self.self_receiver(self.node_cursor(key))?;
                    return Some(match receiver.surface {
                        iris_syntax::MethodKind::Instance | iris_syntax::MethodKind::Property => {
                            TypeFact::Instance(receiver.owner)
                        }
                        iris_syntax::MethodKind::Class | iris_syntax::MethodKind::Module => {
                            TypeFact::Object(receiver.owner)
                        }
                    });
                }
                self.path(self.node_cursor(key), path).map_or_else(
                    || self.builtin_value_type(key, path),
                    |binding| self.binding_type(binding, depth + 1),
                )
            }
            ExpressionFact::Grouped { value } => self.expression_type(
                Key {
                    file: key.file,
                    node: *value,
                },
                depth + 1,
            ),
            ExpressionFact::Member { .. } => self.expression_symbol(key, depth + 1).map_or_else(
                || self.builtin_result(key, depth + 1),
                |target| self.binding_type(target, depth + 1),
            ),
            ExpressionFact::Call {
                callee,
                type_arguments,
                ..
            } => {
                if !type_arguments.is_empty() {
                    return None;
                }
                self.call_type(
                    Key {
                        file: key.file,
                        node: *callee,
                    },
                    depth,
                )
            }
            ExpressionFact::IncompleteMember { .. }
            | ExpressionFact::Closure { .. }
            | ExpressionFact::Assignment { .. }
            | ExpressionFact::Construction { .. }
            | ExpressionFact::ReifiedType { .. }
            | ExpressionFact::KeywordArgument { .. }
            | ExpressionFact::Unsupported { .. } => None,
        }
    }

    fn call_type(&self, callee_key: Key, depth: usize) -> Option<TypeFact> {
        if let SourceKind::Expression(ExpressionFact::Member {
            receiver,
            name,
            contract: false,
        }) = &self.documents[&callee_key.file]
            .source
            .node(callee_key.node)
            .kind
            && name.text == "new"
            && let Some(TypeFact::Object(owner)) = self.expression_type(
                Key {
                    file: callee_key.file,
                    node: *receiver,
                },
                depth + 1,
            )
            && self.symbol(owner)?.declaration.kind == DeclarationKind::Class
            && self
                .symbol(owner)?
                .declaration
                .header
                .as_ref()
                .is_some_and(|header| {
                    header.complete && !header.has_type_parameters && !header.has_decorators
                })
        {
            return Some(TypeFact::Instance(owner));
        }
        let Some(method) = self.expression_symbol(callee_key, depth + 1) else {
            return self.builtin_result(callee_key, depth + 1);
        };
        let symbol = self.symbol(method)?;
        if symbol.declaration.kind != DeclarationKind::Method
                    || symbol.declaration.modifiers.asynchronous
                    || self.documents[&method.file].source.node(method.node).children.iter().any(|child| matches!(&self.documents[&method.file].source.node(*child).kind, SourceKind::Declaration(value) if value.kind == DeclarationKind::TypeParameter))
                {
                    return None;
                }
        symbol.declaration.return_type.map_or(
            Some(TypeFact::Literal("Dynamic<Object>")),
            |annotation| {
                self.annotation_type(Key {
                    file: method.file,
                    node: annotation,
                })
            },
        )
    }

    pub(crate) fn type_label(&self, fact: TypeFact) -> Option<String> {
        match fact {
            TypeFact::Literal(label) => Some(label.to_owned()),
            TypeFact::Written { label } | TypeFact::Builtin { label, .. } => Some(label),
            TypeFact::Instance(key) => Some(self.symbol(key)?.qualified.as_ref()?.join("::")),
            TypeFact::Object(_) | TypeFact::BuiltinClass(_) => None,
        }
    }
}
