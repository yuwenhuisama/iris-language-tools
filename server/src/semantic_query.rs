use iris_analysis::{AnalysisSnapshot, FileId, Span};
use iris_lexer::ByteOffset;
use lsp_types::{
    CompletionItem, CompletionParams, GotoDefinitionParams, InlayHintParams, Position, Range,
    ReferenceParams, Uri,
};
use serde_json::{Value, json};

use crate::{positions::LineIndex, workspace::Snapshot};

#[path = "semantic_completion.rs"]
mod completion;

#[derive(Clone, Debug)]
pub enum Operation {
    Definition(Position),
    References(Position, bool),
    Completion(Position),
    Hints(Range),
}

#[derive(Clone, Debug)]
pub struct Query {
    pub uri: Uri,
    pub operation: Operation,
}

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("position is outside the document or splits a UTF-16 surrogate pair")]
    Position,
    #[error("workspace inventory is incomplete; references cannot be exhaustive")]
    Incomplete,
    #[error("semantic target is outside its source")]
    Target,
}

impl Query {
    pub fn decode(method: &str, params: Value) -> Result<Self, serde_json::Error> {
        match method {
            "textDocument/definition" => {
                let params: GotoDefinitionParams = serde_json::from_value(params)?;
                let position = params.text_document_position_params;
                Ok(Self {
                    uri: position.text_document.uri,
                    operation: Operation::Definition(position.position),
                })
            }
            "textDocument/references" => {
                let params: ReferenceParams = serde_json::from_value(params)?;
                Ok(Self {
                    uri: params.text_document_position.text_document.uri,
                    operation: Operation::References(
                        params.text_document_position.position,
                        params.context.include_declaration,
                    ),
                })
            }
            "textDocument/inlayHint" => {
                let params: InlayHintParams = serde_json::from_value(params)?;
                Ok(Self {
                    uri: params.text_document.uri,
                    operation: Operation::Hints(params.range),
                })
            }
            _ => {
                let params: CompletionParams = serde_json::from_value(params)?;
                Ok(Self {
                    uri: params.text_document_position.text_document.uri,
                    operation: Operation::Completion(params.text_document_position.position),
                })
            }
        }
    }

    pub fn execute(
        &self,
        context: (&Snapshot, &AnalysisSnapshot),
        keywords: &[CompletionItem],
    ) -> Result<Value, QueryError> {
        let (snapshot, analysis) = context;
        if matches!(self.operation, Operation::References(_, _)) && !snapshot.is_complete() {
            return Err(QueryError::Incomplete);
        }
        let Some(source) = snapshot.file(&self.uri) else {
            if snapshot
                .issues
                .iter()
                .any(|issue| issue.uri.as_ref() == Some(&self.uri))
            {
                return Err(QueryError::Incomplete);
            }
            return Ok(match self.operation {
                Operation::Completion(_) => json!(keywords),
                Operation::Definition(_) | Operation::References(_, _) | Operation::Hints(_) => {
                    json!([])
                }
            });
        };
        let file = snapshot
            .files
            .iter()
            .position(|source| source.uri == self.uri)
            .and_then(|index| u32::try_from(index).ok())
            .map(FileId)
            .ok_or(QueryError::Target)?;
        let index = LineIndex::new(&source.text);
        let offset = |position| {
            index
                .byte_offset(position)
                .map(|byte| byte.0)
                .ok_or(QueryError::Position)
        };
        match self.operation {
            Operation::Definition(position) => analysis
                .definitions(file, offset(position)?)
                .into_iter()
                .map(|target| location(snapshot, target.file, target.name_span))
                .collect::<Result<Vec<_>, _>>()
                .map(|locations| json!(locations)),
            Operation::References(position, include_declaration) => {
                let byte = offset(position)?;
                analysis
                    .references(file, byte, include_declaration)
                    .into_iter()
                    .map(|target| location(snapshot, target.file, target.span))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|locations| json!(locations))
            }
            Operation::Completion(position) => {
                let byte = offset(position)?;
                let cursor = index
                    .position(ByteOffset(byte))
                    .ok_or(QueryError::Position)?;
                completion::CompletionContext {
                    index: &index,
                    cursor,
                    inventory_complete: snapshot.is_complete(),
                }
                .response(analysis.completions(file, byte), keywords)
            }
            Operation::Hints(range) => {
                let start = offset(range.start)?;
                let end = offset(range.end)?;
                if start > end {
                    return Err(QueryError::Position);
                }
                analysis
                    .inlay_hints(file, Span { start, end })
                    .into_iter()
                    .filter(|hint| start <= hint.offset && hint.offset < end)
                    .map(|hint| {
                        let position = index
                            .position(ByteOffset(hint.offset))
                            .ok_or(QueryError::Target)?;
                        Ok(json!({"position":position,"label":hint.label,"kind":1}))
                    })
                    .collect::<Result<Vec<Value>, QueryError>>()
                    .map(|hints| json!(hints))
            }
        }
    }
}

fn location(snapshot: &Snapshot, file: FileId, span: Span) -> Result<Value, QueryError> {
    let source = usize::try_from(file.0)
        .ok()
        .and_then(|index| snapshot.files.get(index))
        .ok_or(QueryError::Target)?;
    let range = span_range(&LineIndex::new(&source.text), span)?;
    Ok(json!({"uri":source.uri,"range":range}))
}

fn span_range(index: &LineIndex<'_>, span: Span) -> Result<Range, QueryError> {
    if span.start > span.end {
        return Err(QueryError::Target);
    }
    Ok(Range::new(
        index
            .position(ByteOffset(span.start))
            .ok_or(QueryError::Target)?,
        index
            .position(ByteOffset(span.end))
            .ok_or(QueryError::Target)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{
        CoverageIssue, FileSource, GroupIdentity, IssueKind, Revision, SourceOrigin,
    };
    use iris_analysis::{GroupId, SourceInput};
    use std::sync::Arc;

    #[test]
    fn marks_keyword_response_incomplete_when_workspace_inventory_is_partial() {
        let uri: Uri = "untitled:keywords".parse().unwrap();
        let text: Arc<str> = "let value = 1".into();
        let snapshot = Snapshot {
            revision: Revision(1),
            files: vec![FileSource {
                uri: uri.clone(),
                text: Arc::clone(&text),
                group: GroupIdentity::Standalone(uri.clone()),
                origin: SourceOrigin::Disk,
            }]
            .into(),
            issues: vec![CoverageIssue {
                uri: None,
                path: None,
                kind: IssueKind::FileCount,
            }]
            .into(),
        };
        let analysis = AnalysisSnapshot::new([SourceInput {
            id: FileId(0),
            group: GroupId(0),
            text,
        }]);
        let keywords = [CompletionItem {
            label: "let".into(),
            kind: Some(lsp_types::CompletionItemKind::KEYWORD),
            ..CompletionItem::default()
        }];
        let query = Query {
            uri,
            operation: Operation::Completion(Position::new(0, 0)),
        };

        let when = query.execute((&snapshot, &analysis), &keywords).unwrap();

        assert_eq!(when["isIncomplete"], true);
        assert_eq!(when["items"], json!(keywords));
    }
}
