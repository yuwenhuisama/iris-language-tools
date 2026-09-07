use crate::{
    AnalysisSnapshot, CompletionItem, CompletionKind, CompletionResult, FileId, Span, index::Key,
};
use iris_parser::source::{DeclarationKind, ExpressionFact, SourceKind};
use std::collections::BTreeSet;

impl AnalysisSnapshot {
    #[must_use]
    pub fn completions(&self, file: FileId, byte: usize) -> CompletionResult {
        let Some(cursor) = self.cursor(file, byte) else {
            return CompletionResult::default();
        };
        let document = &self.documents[&file];
        let mut result = CompletionResult {
            items: Vec::new(),
            is_incomplete: !document.source.recovery.is_empty(),
            allow_keywords: false,
        };
        if document.source.nodes.is_empty()
            && document
                .source
                .recovery
                .iter()
                .any(|region| region.code.starts_with("LEX_"))
        {
            result.allow_keywords = document.reliable_keyword_prefix(byte);
            return result;
        }
        if document.completion_protected(byte) || !document.safe_scope(cursor.scope, byte) {
            return result;
        }
        let replace = document.replacement(byte);
        let prefix = &document.input.text[replace.start..byte];
        let mut keys = Vec::new();
        let mut member_context = false;
        for node in &document.source.nodes {
            let receiver = match &node.kind {
                SourceKind::Expression(ExpressionFact::Member {
                    receiver,
                    name,
                    contract,
                }) if name.span.start <= byte && byte <= name.span.end => {
                    member_context = true;
                    if *contract { None } else { Some(*receiver) }
                }
                SourceKind::Expression(ExpressionFact::IncompleteMember { receiver, dot })
                    if dot.end <= byte && document.input.text[dot.end..byte].trim().is_empty() =>
                {
                    member_context = true;
                    Some(*receiver)
                }
                _ => None,
            };
            if let Some(receiver) = receiver.and_then(|receiver| {
                self.receiver(
                    Key {
                        file,
                        node: receiver,
                    },
                    0,
                )
            }) {
                keys.extend(
                    self.member_candidates(cursor, receiver)
                        .into_iter()
                        .filter(|key| {
                            self.symbol(*key).is_some_and(|symbol| {
                                self.member(cursor, receiver, &symbol.declaration.name.text)
                                    == Some(*key)
                            })
                        }),
                );
            }
        }
        if let Some(qualified) = self.qualified_completions(cursor) {
            member_context = true;
            keys.extend(qualified);
        }
        if !member_context {
            result.allow_keywords = document.keyword_context(byte);
            for label in self.completion_labels(file) {
                if !label.starts_with(prefix) {
                    continue;
                }
                if let Some(key) = self
                    .lookup(cursor, &label)
                    .or_else(|| self.implicit_method(cursor, &label))
                    && let Some(mut item) = self.completion_item(key, replace)
                {
                    item.label = label;
                    result.items.push(item);
                }
            }
        }
        for key in keys {
            if let Some(item) = self.completion_item(key, replace)
                && item.label.starts_with(prefix)
            {
                result.items.push(item);
            }
        }
        result
            .items
            .sort_by(|left, right| left.label.cmp(&right.label));
        result
            .items
            .dedup_by(|left, right| left.label == right.label);
        result
    }

    fn completion_labels(&self, file: FileId) -> BTreeSet<String> {
        let mut labels: BTreeSet<_> = self
            .symbols
            .iter()
            .map(|symbol| symbol.declaration.name.text.clone())
            .collect();
        for node in &self.documents[&file].source.nodes {
            if let SourceKind::Import(import) = &node.kind {
                if let Some(alias) = &import.alias {
                    labels.insert(alias.text.clone());
                }
                for spec in &import.specs {
                    labels.insert(spec.alias.as_ref().unwrap_or(&spec.name).text.clone());
                }
            }
        }
        labels
    }

    fn completion_item(&self, key: Key, replace: Span) -> Option<CompletionItem> {
        let symbol = self.symbol(key)?;
        let kind = match symbol.declaration.kind {
            DeclarationKind::Class => CompletionKind::Class,
            DeclarationKind::Module => CompletionKind::Module,
            DeclarationKind::Contract => CompletionKind::Contract,
            DeclarationKind::TypeAlias => CompletionKind::TypeAlias,
            DeclarationKind::Constant => CompletionKind::Constant,
            DeclarationKind::Binding
            | DeclarationKind::Global
            | DeclarationKind::Shared
            | DeclarationKind::PatternBinding => CompletionKind::Variable,
            DeclarationKind::Property => CompletionKind::Property,
            DeclarationKind::Method => CompletionKind::Method,
            DeclarationKind::Parameter => CompletionKind::Parameter,
            DeclarationKind::TypeParameter => CompletionKind::TypeParameter,
        };
        Some(CompletionItem {
            label: symbol.declaration.name.text.clone(),
            detail: self
                .binding_type(key, 0)
                .and_then(|fact| self.type_label(fact)),
            kind,
            replace,
        })
    }
}

impl crate::Document {
    fn replacement(&self, byte: usize) -> Span {
        self.source
            .nodes
            .iter()
            .find_map(|node| match &node.kind {
                SourceKind::Expression(ExpressionFact::Member { name, .. })
                    if name.span.start <= byte && byte <= name.span.end =>
                {
                    Some(name.span)
                }
                _ => None,
            })
            .or_else(|| {
                self.source
                    .tokens
                    .iter()
                    .find(|token| {
                        token.kind == iris_lexer::TokenKind::Identifier
                            && token.offset.0 <= byte
                            && byte <= token.end.0
                    })
                    .map(|token| Span {
                        start: token.offset.0,
                        end: token.end.0,
                    })
            })
            .unwrap_or(Span {
                start: byte,
                end: byte,
            })
    }
}
