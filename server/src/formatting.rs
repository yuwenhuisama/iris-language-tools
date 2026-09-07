use iris_formatter::{FormatOptions, FormatOutcome, format_document};
use iris_lexer::ByteOffset;
use lsp_server::{Request, Response};
use lsp_types::{DocumentFormattingParams, Position, Range, TextEdit};

use crate::{documents::Documents, positions::position};

pub fn respond(request: Request, documents: &Documents) -> Response {
    let params = match serde_json::from_value::<DocumentFormattingParams>(request.params) {
        Ok(params) => params,
        Err(error) => return Response::new_err(request.id, -32602, error.to_string()),
    };
    let Ok(options) = FormatOptions::new(params.options.tab_size, params.options.insert_spaces)
    else {
        return Response::new_err(
            request.id,
            -32602,
            "tabSize must be between 1 and 16".into(),
        );
    };
    let edits = documents
        .get(&params.text_document.uri)
        .map_or_else(Vec::new, |document| {
            match format_document(&document.text, options) {
                FormatOutcome::Changed(new_text) => position(
                    &document.text,
                    ByteOffset(document.text.len()),
                )
                .map_or_else(Vec::new, |end| {
                    vec![TextEdit {
                        range: Range::new(Position::new(0, 0), end),
                        new_text,
                    }]
                }),
                FormatOutcome::Unchanged | FormatOutcome::Skipped(_) => Vec::new(),
            }
        });
    Response::new_ok(request.id, edits)
}
