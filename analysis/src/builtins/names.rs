use crate::{
    AnalysisSnapshot,
    index::{Cursor, Key},
};
use iris_parser::source::DeclarationKind;

impl AnalysisSnapshot {
    pub(crate) fn builtin_unchanged(
        &self,
        file: crate::FileId,
        member: &iris_builtins::BuiltinMember,
    ) -> bool {
        let group = self.documents[&file].input.group;
        if self.incomplete_groups.contains(&group) {
            return false;
        }
        !self.symbols.iter().any(|symbol| {
            symbol.declaration.modifiers.reopen
                && self.documents[&symbol.key.file].input.group == group
                && symbol
                    .declaration
                    .path
                    .iter()
                    .map(|site| site.text.as_str())
                    .collect::<Vec<_>>()
                    .join("::")
                    == member.owner
        })
    }

    pub(crate) fn builtin_selector_available(&self, receiver: Key, name: &str) -> bool {
        let Some(source) = self.receiver(receiver, 0) else {
            return true;
        };
        let Some(owner) = self.symbol(source.owner) else {
            return false;
        };
        let document = &self.documents[&source.owner.file];
        !self.symbols.iter().any(|symbol| {
            symbol.key.file == owner.key.file
                && symbol.declaration.name.text == name
                && document.source.scope(symbol.scope).owner == Some(owner.key.node)
        })
    }

    pub(crate) fn builtin_name_available(&self, cursor: Cursor, name: &str) -> bool {
        if self.has_binding_candidate(cursor, name) || self.lookup(cursor, name).is_some() {
            return false;
        }
        let document = &self.documents[&cursor.file];
        if self.incomplete_groups.contains(&document.input.group) {
            return false;
        }
        if document
            .source
            .recovery
            .iter()
            .any(|region| region.span.start < cursor.offset)
        {
            return false;
        }
        let group = document.input.group;
        let mut scope = Some(cursor.scope);
        let mut paths = vec![vec![name.to_owned()]];
        while let Some(scope_id) = scope {
            let current = document.source.scope(scope_id);
            if !document.safe_scope(scope_id, cursor.offset) {
                return false;
            }
            if self.symbols.iter().any(|symbol| {
                symbol.key.file == cursor.file
                    && symbol.scope == scope_id
                    && symbol.declaration.name.text == name
                    && symbol.declaration.visible_from <= cursor.offset
                    && matches!(
                        symbol.declaration.kind,
                        DeclarationKind::Method | DeclarationKind::Property
                    )
            }) {
                return false;
            }
            if let Some(owner) = current.owner.and_then(|node| {
                self.symbol(Key {
                    file: cursor.file,
                    node,
                })
            }) && let Some(path) = &owner.qualified
            {
                let mut candidate = path.clone();
                candidate.push(name.to_owned());
                paths.push(candidate);
                if owner.declaration.modifiers.reopen
                    || self.declaration_key(owner) != Some(owner.key)
                    || owner.declaration.header.as_ref().is_some_and(|header| {
                        !header.complete
                            || header.has_extends
                            || header.has_mixins
                            || header.has_implements
                            || header.has_decorators
                    })
                {
                    return false;
                }
                if self.symbols.iter().any(|symbol| {
                    self.documents[&symbol.key.file].input.group == group
                        && symbol.declaration.name.text == name
                        && self.documents[&symbol.key.file]
                            .source
                            .scope(symbol.scope)
                            .owner
                            .and_then(|node| {
                                self.symbol(Key {
                                    file: symbol.key.file,
                                    node,
                                })
                            })
                            .and_then(|parent| parent.qualified.as_ref())
                            == Some(path)
                }) {
                    return false;
                }
            }
            scope = current.parent;
        }
        !self.symbols.iter().any(|symbol| {
            self.documents[&symbol.key.file].input.group == group
                && symbol
                    .qualified
                    .as_ref()
                    .is_some_and(|path| paths.iter().any(|candidate| path == candidate))
        })
    }
}
