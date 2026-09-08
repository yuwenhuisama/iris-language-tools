use super::text::BoundedText;
use crate::{AnalysisSnapshot, DocumentationInfo, HoverDetail, index::Key};
use iris_parser::source::{DeclarationKind, SourceKind};

impl AnalysisSnapshot {
    pub(crate) fn documentation(&self, key: Key) -> Option<DocumentationInfo> {
        let docs = self.documents[&key.file]
            .source
            .documentation
            .iter()
            .find(|docs| docs.declaration == key.node)?;
        Some(DocumentationInfo {
            text: docs.text.clone(),
            truncated: docs.truncated,
        })
    }

    pub(super) fn hover_owner(&self, key: Key) -> Option<String> {
        let symbol = self.symbol(key)?;
        let document = &self.documents[&key.file];
        if let Some(path) = &symbol.qualified
            && path.len() > 1
        {
            let owner = self.namespace(self.node_cursor(key), &path[..path.len() - 1])?;
            let mut text = BoundedText::new(1024);
            for (index, segment) in self.symbol(owner)?.qualified.as_ref()?.iter().enumerate() {
                if index > 0 {
                    text.push("::");
                }
                text.push(segment);
                if text.full() {
                    break;
                }
            }
            return Some(text.finish());
        }
        let mut scope = document.source.scope(symbol.scope);
        let mut owners = Vec::new();
        loop {
            if let Some(owner) = scope.owner
                && let SourceKind::Declaration(declaration) = &document.source.node(owner).kind
            {
                owners.push(&declaration.path);
            }
            match scope.parent {
                Some(parent) => scope = document.source.scope(parent),
                None => break,
            }
        }
        let mut text = BoundedText::new(1024);
        let mut first = true;
        for site in owners.iter().rev().flat_map(|path| path.iter()) {
            if !first {
                text.push("::");
            }
            text.push(&site.text);
            first = false;
            if text.full() {
                break;
            }
        }
        (!first).then(|| text.finish())
    }

    pub(super) fn hover_details(&self, key: Key) -> Vec<HoverDetail> {
        let Some(symbol) = self.symbol(key) else {
            return Vec::new();
        };
        let mut details = Vec::new();
        if let Some(label) = self.hover_label(key, 0) {
            details.push(match symbol.declaration.kind {
                DeclarationKind::Method => HoverDetail::ReturnType(label),
                DeclarationKind::Class
                | DeclarationKind::Module
                | DeclarationKind::Contract
                | DeclarationKind::Binding
                | DeclarationKind::Constant
                | DeclarationKind::TypeAlias
                | DeclarationKind::Global
                | DeclarationKind::Shared
                | DeclarationKind::Property
                | DeclarationKind::Parameter
                | DeclarationKind::TypeParameter
                | DeclarationKind::PatternBinding => HoverDetail::ValueType(label),
            });
        }
        if let Some(category) = symbol.declaration.parameter_category {
            details.push(HoverDetail::ParameterCategory(category));
        }
        details
    }
}
