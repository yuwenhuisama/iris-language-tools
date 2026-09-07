use crate::{
    AnalysisSnapshot,
    index::{Cursor, Key, Symbol, captures, namespace_kind},
};
use iris_parser::source::{DeclarationKind, NameSite, ScopeKind, SourceKind};

impl AnalysisSnapshot {
    pub(crate) fn has_binding_candidate(&self, cursor: Cursor, name: &str) -> bool {
        let document = &self.documents[&cursor.file];
        let mut scope = document.source.scope(cursor.scope);
        let mut crossed_method = false;
        loop {
            if !document.safe_scope(scope.id, cursor.offset) {
                return true;
            }
            if self.symbols.iter().any(|symbol| {
                symbol.key.file == cursor.file
                    && symbol.scope == scope.id
                    && symbol.declaration.name.text == name
                    && symbol.declaration.visible_from <= cursor.offset
                    && !(crossed_method && captures(symbol.declaration.kind))
                    && !matches!(
                        symbol.declaration.kind,
                        DeclarationKind::Method
                            | DeclarationKind::Property
                            | DeclarationKind::Shared
                            | DeclarationKind::Global
                    )
            }) {
                return true;
            }
            if scope.kind == ScopeKind::Method {
                crossed_method = true;
            }
            match scope.parent {
                Some(parent) => scope = document.source.scope(parent),
                None => break,
            }
        }
        document.source.nodes.iter().any(|node| match &node.kind {
            SourceKind::Import(import) => {
                import
                    .alias
                    .as_ref()
                    .or_else(|| import.target.last())
                    .is_some_and(|site| site.text == name)
                    || import
                        .specs
                        .iter()
                        .any(|spec| spec.alias.as_ref().unwrap_or(&spec.name).text == name)
            }
            _ => false,
        })
    }

    pub(crate) fn unique<'a>(mut symbols: impl Iterator<Item = &'a Symbol>) -> Option<Key> {
        let first = symbols.next()?;
        if symbols.next().is_some() || first.declaration.modifiers.reopen {
            return None;
        }
        Some(first.key)
    }

    pub(crate) fn lookup(&self, cursor: Cursor, name: &str) -> Option<Key> {
        let document = &self.documents[&cursor.file];
        let mut scope = document.source.scope(cursor.scope);
        let mut crossed_method = false;
        loop {
            if !document.safe_scope(scope.id, cursor.offset) {
                return None;
            }
            let candidates: Vec<_> = self
                .symbols
                .iter()
                .filter(|symbol| {
                    symbol.key.file == cursor.file
                        && symbol.scope == scope.id
                        && symbol.declaration.name.text == name
                        && symbol.declaration.path.len() == 1
                        && !(crossed_method && captures(symbol.declaration.kind))
                        && !matches!(
                            symbol.declaration.kind,
                            DeclarationKind::Method
                                | DeclarationKind::Property
                                | DeclarationKind::Global
                                | DeclarationKind::Shared
                        )
                })
                .collect();
            let active: Vec<_> = candidates
                .iter()
                .copied()
                .filter(|symbol| symbol.declaration.visible_from <= cursor.offset)
                .collect();
            if !active.is_empty() {
                if candidates.len() != 1 {
                    return None;
                }
                let key = Self::unique(active.into_iter())?;
                let symbol = self.symbol(key)?;
                if let Some(path) = &symbol.qualified {
                    return self.namespace(cursor, path);
                }
                return Some(key);
            }
            if scope.kind == ScopeKind::Method {
                crossed_method = true;
            }
            match scope.parent {
                Some(parent) => scope = document.source.scope(parent),
                None => break,
            }
        }
        let mut owner_scope = document.source.scope(cursor.scope);
        loop {
            if let Some(owner) = owner_scope.owner
                && let Some(target) = self.namespace_child(
                    cursor,
                    Key {
                        file: cursor.file,
                        node: owner,
                    },
                    name,
                )
            {
                return Some(target);
            }
            match owner_scope.parent {
                Some(parent) => owner_scope = document.source.scope(parent),
                None => break,
            }
        }
        let path = [name.to_owned()];
        let group = document.input.group;
        if self.symbols.iter().any(|symbol| {
            self.documents[&symbol.key.file].input.group == group
                && symbol.qualified.as_deref() == Some(&path)
        }) {
            return self.namespace(cursor, &path);
        }
        self.import_alias(cursor, name)
    }

    pub(crate) fn namespace(&self, cursor: Cursor, path: &[String]) -> Option<Key> {
        let group = self.documents[&cursor.file].input.group;
        Self::unique(self.symbols.iter().filter(|symbol| {
            self.documents[&symbol.key.file].input.group == group
                && symbol.qualified.as_deref() == Some(path)
                && self.documents[&symbol.key.file]
                    .safe_scope(symbol.scope, symbol.declaration.name.span.end)
        }))
    }

    pub(crate) fn path(&self, cursor: Cursor, path: &[NameSite]) -> Option<Key> {
        let first = path.first()?;
        let mut key = self.lookup(cursor, &first.text)?;
        for site in &path[1..] {
            key = self.namespace_child(cursor, key, &site.text)?;
        }
        Some(key)
    }

    pub(crate) fn namespace_child(&self, cursor: Cursor, owner: Key, name: &str) -> Option<Key> {
        let mut path = self.symbol(owner)?.qualified.clone()?;
        path.push(name.to_owned());
        self.namespace(cursor, &path)
    }

    fn import_alias(&self, cursor: Cursor, name: &str) -> Option<Key> {
        let document = &self.documents[&cursor.file];
        let mut scope = Some(cursor.scope);
        while let Some(scope_id) = scope {
            let mut candidates = Vec::new();
            for node in &document.source.nodes {
                let SourceKind::Import(import) = &node.kind else {
                    continue;
                };
                if node.scope != scope_id {
                    continue;
                }
                if import.specs.is_empty() {
                    if import
                        .alias
                        .as_ref()
                        .or_else(|| import.target.last())
                        .is_some_and(|site| site.text == name)
                    {
                        candidates.push(self.import_target(cursor, &import.target));
                    }
                } else {
                    for spec in &import.specs {
                        if spec.alias.as_ref().unwrap_or(&spec.name).text == name {
                            candidates.push(self.import_target(cursor, &import.target).and_then(
                                |owner| self.namespace_child(cursor, owner, &spec.name.text),
                            ));
                        }
                    }
                }
            }
            if candidates.len() == 1 {
                return candidates[0];
            }
            if !candidates.is_empty() {
                return None;
            }
            scope = document.source.scope(scope_id).parent;
        }
        None
    }

    pub(crate) fn import_target(&self, cursor: Cursor, path: &[NameSite]) -> Option<Key> {
        let text = &self.documents[&cursor.file].input.text;
        if path
            .windows(2)
            .any(|pair| text[pair[0].span.end..pair[1].span.start].contains('.'))
        {
            return None;
        }
        let names: Vec<_> = path.iter().map(|site| site.text.clone()).collect();
        self.namespace(cursor, &names)
    }

    pub(crate) fn declaration_key(&self, symbol: &Symbol) -> Option<Key> {
        if namespace_kind(symbol.declaration.kind) {
            return self.namespace(self.node_cursor(symbol.key), symbol.qualified.as_ref()?);
        }
        Self::unique(self.symbols.iter().filter(|candidate| {
            candidate.key.file == symbol.key.file
                && candidate.scope == symbol.scope
                && candidate.declaration.name.text == symbol.declaration.name.text
                && candidate.declaration.surface == symbol.declaration.surface
        }))
    }
}
