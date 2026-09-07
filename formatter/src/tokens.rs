use iris_lexer::TokenKind;

use crate::SkipReason;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Code,
    Literal,
    Comment,
    Newline,
}

#[derive(Clone, Copy, Debug)]
pub struct Piece<'a> {
    pub text: &'a str,
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
}

pub fn scan(source: &str) -> Result<Vec<Piece<'_>>, SkipReason> {
    let lexed = iris_lexer::lex(source.as_bytes());
    if !lexed.is_clean() {
        return Err(SkipReason::LexicalDiagnostics);
    }
    let mut pieces = Vec::new();
    let mut end = source.strip_prefix('\u{feff}').map_or(0, |_| 3);
    for token in lexed.tokens() {
        if token.kind == TokenKind::Newline {
            continue;
        }
        trivia(&source[end..token.offset.0], &mut pieces)?;
        let kind = match token.kind {
            TokenKind::StringLiteral
            | TokenKind::MutableStringLiteral
            | TokenKind::BytesLiteral
            | TokenKind::ByteArrayLiteral
            | TokenKind::RegexLiteral => Kind::Literal,
            _ => Kind::Code,
        };
        pieces.push(Piece {
            text: &source[token.offset.0..token.end.0],
            kind,
            start: token.offset.0,
            end: token.end.0,
        });
        end = token.end.0;
    }
    trivia(&source[end..], &mut pieces)?;
    Ok(pieces)
}

fn trivia<'a>(mut text: &'a str, pieces: &mut Vec<Piece<'a>>) -> Result<(), SkipReason> {
    while !text.is_empty() {
        let width;
        if text.starts_with([' ', '\t']) {
            width = text.len() - text.trim_start_matches([' ', '\t']).len();
        } else if text.starts_with(['\r', '\n']) {
            width = if text.starts_with("\r\n") { 2 } else { 1 };
            if !pieces
                .last()
                .is_some_and(|piece| piece.kind == Kind::Newline)
            {
                pieces.push(Piece {
                    text: "\n",
                    kind: Kind::Newline,
                    start: 0,
                    end: 0,
                });
            }
        } else if text.starts_with("//") || text.starts_with("#!") {
            width = text.find(['\r', '\n']).unwrap_or(text.len());
            pieces.push(Piece {
                text: &text[..width],
                kind: Kind::Comment,
                start: 0,
                end: 0,
            });
        } else if text.starts_with("/*") {
            let bytes = text.as_bytes();
            let mut cursor = 2;
            let mut depth = 1;
            while depth > 0 && cursor + 1 < bytes.len() {
                match &bytes[cursor..cursor + 2] {
                    b"/*" => {
                        depth += 1;
                        cursor += 2;
                    }
                    b"*/" => {
                        depth -= 1;
                        cursor += 2;
                    }
                    _ => cursor += 1,
                }
            }
            if depth != 0 {
                return Err(SkipReason::UncertainProtectedRange);
            }
            width = cursor;
            pieces.push(Piece {
                text: &text[..width],
                kind: Kind::Comment,
                start: 0,
                end: 0,
            });
        } else if text.starts_with("\\\n") || text.starts_with("\\\r") {
            return Err(SkipReason::Continuation);
        } else {
            return Err(SkipReason::UncertainProtectedRange);
        }
        text = &text[width..];
    }
    Ok(())
}

pub fn comment(text: &str) -> String {
    let marker = if text.starts_with("///") { "///" } else { "//" };
    text.strip_prefix(marker).map_or_else(
        || text.into(),
        |body| {
            if body.trim_start_matches([' ', '\t']).is_empty() {
                marker.into()
            } else {
                format!("{marker} {}", body.trim_start_matches([' ', '\t']))
            }
        },
    )
}

pub fn verify(original: &[Piece<'_>], candidate: &[Piece<'_>]) -> Result<(), SkipReason> {
    let protected = |pieces: &[Piece<'_>]| -> Vec<(Kind, String)> {
        pieces
            .iter()
            .filter_map(|piece| match piece.kind {
                Kind::Literal => Some((Kind::Literal, piece.text.into())),
                Kind::Comment => Some((Kind::Comment, comment(piece.text))),
                Kind::Code | Kind::Newline => None,
            })
            .collect()
    };
    if protected(original) != protected(candidate) {
        return Err(SkipReason::TokenMismatch);
    }
    let significant = |pieces: &[Piece<'_>]| -> Vec<String> {
        pieces
            .iter()
            .filter(|piece| piece.kind == Kind::Code && !matches!(piece.text, ";" | ","))
            .map(|piece| piece.text.into())
            .collect()
    };
    if significant(original) != significant(candidate) {
        return Err(SkipReason::TokenMismatch);
    }
    Ok(())
}
