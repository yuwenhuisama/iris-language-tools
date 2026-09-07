use std::collections::BTreeSet;

use iris_analysis::{CompletionKind, CompletionResult};
use lsp_types::{CompletionItem, CompletionItemKind, Position};
use serde_json::{Value, json};

use super::{QueryError, span_range};
use crate::positions::LineIndex;

pub(super) struct CompletionContext<'index, 'text> {
    pub index: &'index LineIndex<'text>,
    pub cursor: Position,
    pub inventory_complete: bool,
}

impl CompletionContext<'_, '_> {
    pub fn response(
        &self,
        result: CompletionResult,
        keywords: &[CompletionItem],
    ) -> Result<Value, QueryError> {
        let keyword_only = result.items.is_empty() && result.allow_keywords;
        let mut items = Vec::new();
        let mut labels = BTreeSet::new();
        for item in result.items {
            let range = span_range(self.index, item.replace)?;
            if range.start.line != range.end.line
                || !(range.start..=range.end).contains(&self.cursor)
            {
                return Err(QueryError::Target);
            }
            labels.insert(item.label.clone());
            items.push(
                json!({"label":item.label,"detail":item.detail,"kind":completion_kind(item.kind),
                "textEdit":{"range":range,"newText":item.label}}),
            );
        }
        if result.allow_keywords {
            items.extend(
                keywords
                    .iter()
                    .filter(|item| labels.insert(item.label.clone()))
                    .map(|item| json!(item)),
            );
        }
        if keyword_only && self.inventory_complete {
            return Ok(json!(items));
        }
        Ok(json!({"isIncomplete":result.is_incomplete || !self.inventory_complete, "items":items}))
    }
}

const fn completion_kind(kind: CompletionKind) -> CompletionItemKind {
    match kind {
        CompletionKind::Variable | CompletionKind::Parameter => CompletionItemKind::VARIABLE,
        CompletionKind::Constant => CompletionItemKind::CONSTANT,
        CompletionKind::TypeParameter => CompletionItemKind::TYPE_PARAMETER,
        CompletionKind::Class | CompletionKind::TypeAlias => CompletionItemKind::CLASS,
        CompletionKind::Module => CompletionItemKind::MODULE,
        CompletionKind::Contract => CompletionItemKind::INTERFACE,
        CompletionKind::Method => CompletionItemKind::METHOD,
        CompletionKind::Property => CompletionItemKind::PROPERTY,
    }
}
