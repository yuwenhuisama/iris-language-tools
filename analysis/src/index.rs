use crate::{AnalysisSnapshot, Document, FileId, Span};
use iris_parser::source::{
    DeclarationKind, ScopeId, ScopeKind, SourceDeclaration, SourceKind, SyntaxId,
};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Key {
    pub file: FileId,
    pub node: SyntaxId,
}

#[derive(Clone, Copy)]
pub struct Cursor {
    pub file: FileId,
    pub scope: ScopeId,
    pub offset: usize,
}

#[derive(Debug)]
pub struct Symbol {
    pub key: Key,
    pub declaration: SourceDeclaration,
    pub scope: ScopeId,
    pub span: Span,
    pub qualified: Option<Vec<String>>,
}

impl AnalysisSnapshot {
    pub(crate) fn collect_symbols(&self) -> Vec<Symbol> {
        self.documents
            .iter()
            .flat_map(|(file, document)| {
                document.source.nodes.iter().filter_map(move |node| {
                    let SourceKind::Declaration(declaration) = &node.kind else {
                        return None;
                    };
                    let qualified = namespace_kind(declaration.kind)
                        .then(|| {
                            let mut owners = Vec::new();
                            let mut scope = document.source.scope(node.scope);
                            while let Some(owner) = scope.owner {
                                match &document.source.node(owner).kind {
                                    SourceKind::Declaration(value)
                                        if namespace_kind(value.kind) =>
                                    {
                                        owners.push(
                                            value
                                                .path
                                                .iter()
                                                .map(|site| site.text.clone())
                                                .collect::<Vec<_>>(),
                                        );
                                    }
                                    _ => return None,
                                }
                                scope = document.source.scope(scope.parent?);
                            }
                            owners.reverse();
                            let mut path: Vec<_> = owners.into_iter().flatten().collect();
                            path.extend(declaration.path.iter().map(|site| site.text.clone()));
                            Some(path)
                        })
                        .flatten();
                    Some(Symbol {
                        key: Key {
                            file: *file,
                            node: node.id,
                        },
                        declaration: declaration.as_ref().clone(),
                        scope: node.scope,
                        span: node.span,
                        qualified,
                    })
                })
            })
            .collect()
    }

    pub(crate) fn symbol(&self, key: Key) -> Option<&Symbol> {
        self.symbols.iter().find(|symbol| symbol.key == key)
    }

    pub(crate) fn cursor(&self, file: FileId, offset: usize) -> Option<Cursor> {
        let document = self.documents.get(&file)?;
        if offset > document.input.text.len() || !document.input.text.is_char_boundary(offset) {
            return None;
        }
        let scope = document
            .source
            .scopes
            .iter()
            .filter(|scope| scope.span.start <= offset && offset <= scope.span.end)
            .min_by_key(|scope| {
                (
                    scope.span.end - scope.span.start,
                    std::cmp::Reverse(scope.id.0),
                )
            })?;
        Some(Cursor {
            file,
            scope: scope.id,
            offset,
        })
    }

    pub(crate) fn node_cursor(&self, key: Key) -> Cursor {
        let node = self.documents[&key.file].source.node(key.node);
        Cursor {
            file: key.file,
            scope: node.scope,
            offset: node.span.start,
        }
    }
}

impl Document {
    pub(crate) fn safe_scope(&self, scope: ScopeId, offset: usize) -> bool {
        if !self.source.scope(scope).damaged {
            return true;
        }
        let recovery: Vec<_> = self
            .source
            .recovery
            .iter()
            .filter(|region| region.scope == scope)
            .collect();
        !recovery.is_empty() && recovery.iter().all(|region| region.span.start >= offset)
    }

    pub(crate) fn protected(&self, offset: usize) -> bool {
        self.source.protected.iter().any(|span| {
            span.start <= offset
                && (offset < span.end || offset == self.input.text.len() && offset == span.end)
        })
    }

    pub(crate) fn completion_protected(&self, offset: usize) -> bool {
        self.protected(offset)
            || self.source.nodes.iter().any(|node| {
                matches!(
                    node.kind,
                    SourceKind::Expression(iris_parser::source::ExpressionFact::Literal { .. })
                ) && node.span.start <= offset
                    && offset <= node.span.end
            })
    }
}

pub const fn namespace_kind(kind: DeclarationKind) -> bool {
    match kind {
        DeclarationKind::Class
        | DeclarationKind::Module
        | DeclarationKind::Contract
        | DeclarationKind::TypeAlias
        | DeclarationKind::Constant => true,
        DeclarationKind::Binding
        | DeclarationKind::Global
        | DeclarationKind::Shared
        | DeclarationKind::Property
        | DeclarationKind::Method
        | DeclarationKind::Parameter
        | DeclarationKind::TypeParameter
        | DeclarationKind::PatternBinding => false,
    }
}

pub const fn captures(kind: DeclarationKind) -> bool {
    match kind {
        DeclarationKind::Binding | DeclarationKind::Parameter | DeclarationKind::PatternBinding => {
            true
        }
        DeclarationKind::Class
        | DeclarationKind::Module
        | DeclarationKind::Contract
        | DeclarationKind::TypeAlias
        | DeclarationKind::Constant
        | DeclarationKind::Global
        | DeclarationKind::Shared
        | DeclarationKind::Property
        | DeclarationKind::Method
        | DeclarationKind::TypeParameter => false,
    }
}

pub const fn owner_scope(kind: ScopeKind) -> bool {
    match kind {
        ScopeKind::Class | ScopeKind::Module | ScopeKind::Contract => true,
        ScopeKind::Document
        | ScopeKind::TypeAlias
        | ScopeKind::Method
        | ScopeKind::Closure
        | ScopeKind::Block
        | ScopeKind::Loop
        | ScopeKind::MatchArm
        | ScopeKind::Catch => false,
    }
}
