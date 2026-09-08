use iris_analysis::{AnalysisSnapshot, FileId, Span};
use iris_lexer::ByteOffset;
use lsp_types::{
    CompletionItem, CompletionParams, GotoDefinitionParams, HoverParams, InlayHintParams, Position,
    Range, ReferenceParams, SignatureHelpParams, Uri,
};
use serde_json::{Value, json};

use crate::{
    hover::{self, Format},
    positions::LineIndex,
    signature_help::{self, Options},
    workspace::{InventoryDetails, Snapshot},
};

#[path = "semantic_completion.rs"]
mod completion;

#[derive(Clone, Debug)]
pub enum Operation {
    Definition(Position),
    References(Position, bool),
    Completion(Position),
    Hints(Range),
    Hover(Position, Format),
    SignatureHelp(Position, Options),
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
    Incomplete(InventoryDetails),
    #[error("semantic target is outside its source")]
    Target,
}

impl Query {
    pub fn decode(
        method: &str,
        params: Value,
        options: (Format, Options),
    ) -> Result<Self, serde_json::Error> {
        match method {
            "textDocument/signatureHelp" => {
                let params: SignatureHelpParams = serde_json::from_value(params)?;
                let position = params.text_document_position_params;
                Ok(Self {
                    uri: position.text_document.uri,
                    operation: Operation::SignatureHelp(position.position, options.1),
                })
            }
            "textDocument/hover" => {
                let params: HoverParams = serde_json::from_value(params)?;
                let position = params.text_document_position_params;
                Ok(Self {
                    uri: position.text_document.uri,
                    operation: Operation::Hover(position.position, options.0),
                })
            }
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
            return Err(QueryError::Incomplete(InventoryDetails::from_issues(
                &snapshot.issues,
            )));
        }
        let Some(source) = snapshot.file(&self.uri) else {
            if snapshot
                .issues
                .iter()
                .any(|issue| issue.uri.as_ref() == Some(&self.uri))
            {
                return Err(QueryError::Incomplete(InventoryDetails::from_issues(
                    &snapshot.issues,
                )));
            }
            return Ok(match self.operation {
                Operation::Completion(_) => json!(keywords),
                Operation::Hover(_, _) | Operation::SignatureHelp(_, _) => Value::Null,
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
            Operation::SignatureHelp(position, options) => Ok(json!(
                analysis
                    .signature_help(file, offset(position)?)
                    .and_then(|info| signature_help::render(info, options))
            )),
            Operation::Hover(position, format) => {
                let Some(info) = analysis.hover(file, offset(position)?) else {
                    return Ok(Value::Null);
                };
                let range = span_range(&index, info.span)?;
                Ok(json!(lsp_types::Hover {
                    contents: lsp_types::HoverContents::Markup(hover::render(&info, format)),
                    range: Some(range),
                }))
            }
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
                completion::CompletionContext {
                    index: &index,
                    cursor: index
                        .position(ByteOffset(byte))
                        .ok_or(QueryError::Position)?,
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
#[path = "semantic_query_tests.rs"]
mod tests;
