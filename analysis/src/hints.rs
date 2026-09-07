use crate::{AnalysisSnapshot, FileId, Span, TypeHint};
use iris_parser::source::{DeclarationKind, ScopeKind};

impl AnalysisSnapshot {
    #[must_use]
    pub fn inlay_hints(&self, file: FileId, range: Span) -> Vec<TypeHint> {
        let Some(document) = self.documents.get(&file) else {
            return Vec::new();
        };
        let mut hints = Vec::new();
        for symbol in self.symbols.iter().filter(|symbol| symbol.key.file == file) {
            let declaration = &symbol.declaration;
            if self.declaration_key(symbol) != Some(symbol.key)
                || !document.safe_scope(symbol.scope, declaration.name.span.end)
            {
                continue;
            }
            match declaration.kind {
                DeclarationKind::Parameter
                    if declaration.annotation.is_none()
                        && document.source.scope(symbol.scope).kind == ScopeKind::Method =>
                {
                    hints.push(TypeHint {
                        offset: declaration.name.span.end,
                        label: ": Dynamic<Object>".to_owned(),
                    });
                }
                DeclarationKind::Binding if declaration.annotation.is_none() => {
                    if let Some(label) = self
                        .binding_type(symbol.key, 0)
                        .and_then(|fact| self.type_label(fact))
                    {
                        hints.push(TypeHint {
                            offset: declaration.name.span.end,
                            label: format!(": {label}"),
                        });
                    }
                }
                DeclarationKind::Method if declaration.return_type.is_none() => {
                    if let Some(offset) = declaration.return_hint_offset {
                        hints.push(TypeHint {
                            offset,
                            label: " -> Dynamic<Object>".to_owned(),
                        });
                    }
                }
                DeclarationKind::Class
                | DeclarationKind::Module
                | DeclarationKind::Contract
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
        hints.retain(|hint| range.start <= hint.offset && hint.offset < range.end);
        hints.sort_by_key(|hint| hint.offset);
        hints
    }
}
