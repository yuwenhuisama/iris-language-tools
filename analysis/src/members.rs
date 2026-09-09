use crate::{
    AnalysisSnapshot,
    index::{Cursor, Key, owner_scope},
    types::TypeFact,
};
use iris_parser::source::{DeclarationKind, ExpressionFact, ScopeKind, SourceKind};
use iris_syntax::{MethodKind, Visibility};

#[derive(Clone, Copy)]
pub struct Receiver {
    pub owner: Key,
    pub surface: MethodKind,
}

impl AnalysisSnapshot {
    pub(crate) fn receiver(&self, key: Key, depth: usize) -> Option<Receiver> {
        match self.expression_type(key, depth + 1)? {
            TypeFact::Instance(owner) => Some(Receiver {
                owner,
                surface: MethodKind::Instance,
            }),
            TypeFact::Object(owner) => {
                let surface = match self.symbol(owner)?.declaration.kind {
                    DeclarationKind::Class => MethodKind::Class,
                    DeclarationKind::Module => MethodKind::Module,
                    DeclarationKind::Contract
                    | DeclarationKind::TypeAlias
                    | DeclarationKind::Binding
                    | DeclarationKind::Constant
                    | DeclarationKind::Global
                    | DeclarationKind::Shared
                    | DeclarationKind::Property
                    | DeclarationKind::Method
                    | DeclarationKind::Parameter
                    | DeclarationKind::TypeParameter
                    | DeclarationKind::PatternBinding => return None,
                };
                Some(Receiver { owner, surface })
            }
            TypeFact::Written { .. }
            | TypeFact::ArrayOf(_)
            | TypeFact::Nullable(_)
            | TypeFact::Literal(_)
            | TypeFact::Builtin { .. }
            | TypeFact::BuiltinClass(_) => None,
        }
    }

    pub(crate) fn expression_symbol(&self, key: Key, depth: usize) -> Option<Key> {
        if depth > 64 {
            return None;
        }
        let node = self.documents[&key.file].source.node(key.node);
        let cursor = self.node_cursor(key);
        match &node.kind {
            SourceKind::Expression(ExpressionFact::Name { path }) => {
                self.path(cursor, path).or_else(|| {
                    let parent = self.documents[&key.file].parents[key.node.0]?;
                    let SourceKind::Expression(ExpressionFact::Call { callee, .. }) =
                        self.documents[&key.file].source.node(parent).kind
                    else {
                        return None;
                    };
                    if callee != key.node || path.len() != 1 {
                        return None;
                    }
                    self.implicit_method(cursor, &path[0].text)
                })
            }
            SourceKind::Expression(ExpressionFact::Member {
                receiver,
                name,
                contract,
            }) => {
                if *contract {
                    return None;
                }
                let receiver = self.receiver(
                    Key {
                        file: key.file,
                        node: *receiver,
                    },
                    depth + 1,
                )?;
                self.member(cursor, receiver, &name.text)
            }
            SourceKind::Expression(ExpressionFact::Grouped { value }) => self.expression_symbol(
                Key {
                    file: key.file,
                    node: *value,
                },
                depth + 1,
            ),
            SourceKind::Name(_)
            | SourceKind::Declaration(_)
            | SourceKind::Type(_)
            | SourceKind::Import(_)
            | SourceKind::Export
            | SourceKind::Body
            | SourceKind::Statement
            | SourceKind::Pattern
            | SourceKind::Decorator
            | SourceKind::Expression(_) => None,
        }
    }

    pub(crate) fn implicit_method(&self, cursor: Cursor, name: &str) -> Option<Key> {
        if self.has_binding_candidate(cursor, name) {
            return None;
        }
        self.member(cursor, self.self_receiver(cursor)?, name)
    }

    pub(crate) fn self_receiver(&self, cursor: Cursor) -> Option<Receiver> {
        let document = &self.documents[&cursor.file];
        let mut scope = document.source.scope(cursor.scope);
        let mut surface = None;
        loop {
            if !document.safe_scope(scope.id, cursor.offset) {
                return None;
            }
            if scope.kind == ScopeKind::Method {
                let symbol = self.symbol(Key {
                    file: cursor.file,
                    node: scope.owner?,
                })?;
                surface = symbol.declaration.surface;
            }
            if owner_scope(scope.kind) {
                let owner = Key {
                    file: cursor.file,
                    node: scope.owner?,
                };
                return Some(Receiver {
                    owner,
                    surface: surface.unwrap_or(match scope.kind {
                        ScopeKind::Class => MethodKind::Class,
                        _ => MethodKind::Instance,
                    }),
                });
            }
            scope = document.source.scope(scope.parent?);
        }
    }

    pub(crate) fn member(&self, cursor: Cursor, receiver: Receiver, name: &str) -> Option<Key> {
        let candidates = self.member_candidates(cursor, receiver);
        Self::unique(
            candidates
                .into_iter()
                .filter_map(|key| self.symbol(key))
                .filter(|symbol| symbol.declaration.name.text == name),
        )
    }

    pub(crate) fn member_candidates(&self, cursor: Cursor, receiver: Receiver) -> Vec<Key> {
        let Some(owner) = self.symbol(receiver.owner) else {
            return Vec::new();
        };
        if self.declaration_key(owner) != Some(owner.key) {
            return Vec::new();
        }
        let document = &self.documents[&owner.key.file];
        if owner.declaration.header.as_ref().is_none_or(|header| {
            !header.complete
                || header.has_extends
                || header.has_implements
                || header.has_mixins
                || header.has_type_parameters
                || header.has_constraints
                || header.has_decorators
        }) {
            return Vec::new();
        }
        let Some(member_scope) = document
            .source
            .scopes
            .iter()
            .find(|scope| scope.owner == Some(owner.key.node) && owner_scope(scope.kind))
        else {
            return Vec::new();
        };
        let lexical_owner = self.self_receiver(cursor).map(|value| value.owner);
        let scope_id = member_scope.id;
        self.symbols
            .iter()
            .filter(|symbol| {
                let declaration = &symbol.declaration;
                symbol.key.file == owner.key.file
                    && symbol.scope == scope_id
                    && matches!(
                        declaration.kind,
                        DeclarationKind::Method | DeclarationKind::Property
                    )
                    && !declaration.modifiers.implementation
                    && match declaration.surface {
                        Some(MethodKind::Property | MethodKind::Instance) => {
                            receiver.surface == MethodKind::Instance
                        }
                        Some(MethodKind::Class) => receiver.surface == MethodKind::Class,
                        Some(MethodKind::Module) => receiver.surface == MethodKind::Module,
                        None => false,
                    }
                    && match declaration.visibility {
                        Visibility::Public => true,
                        Visibility::Private | Visibility::Protected => {
                            lexical_owner == Some(owner.key)
                        }
                    }
                    && document.safe_scope(member_scope.id, declaration.name.span.end)
                    && self.declaration_key(symbol) == Some(symbol.key)
            })
            .map(|symbol| symbol.key)
            .collect()
    }
}
