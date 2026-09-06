use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Range};

use crate::positions::position;

pub fn analyze(text: &str) -> (Vec<Diagnostic>, Vec<&'static str>) {
    let lexed = iris_lexer::lex(text.as_bytes());
    let mut located = Vec::new();
    let mut unlocated = Vec::new();
    for diagnostic in lexed.diagnostics() {
        let code = diagnostic.code();
        let precise = matches!(
            code,
            "LEX_SHEBANG_NOT_FIRST"
                | "LEX_UNTERMINATED_COMMENT"
                | "LEX_BAD_CONTINUATION"
                | "LEX_INTERPOLATION_OUTSIDE_LITERAL"
                | "LEX_UNTERMINATED_LITERAL"
                | "LEX_INVALID_IDENTIFIER"
                | "LEX_BAD_LITERAL_PREFIX"
        );
        match precise
            .then(|| position(text, diagnostic.offset()))
            .flatten()
        {
            Some(start) => located.push(Diagnostic {
                range: Range::new(start, start),
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String(code.into())),
                source: Some("iris-lexer".into()),
                message: code.into(),
                ..Diagnostic::default()
            }),
            None => unlocated.push(code),
        }
    }
    (located, unlocated)
}
