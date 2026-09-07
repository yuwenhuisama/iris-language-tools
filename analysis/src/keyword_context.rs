use crate::Document;
use iris_lexer::{Token, TokenKind};

impl Document {
    pub(crate) fn keyword_context(&self, byte: usize) -> bool {
        lexical_context(&self.input.text, &self.source.tokens, byte)
    }

    pub(crate) fn reliable_keyword_prefix(&self, byte: usize) -> bool {
        let end = self.input.text[byte..]
            .chars()
            .next()
            .map_or(byte, |character| byte + character.len_utf8());
        if end > 64 * 1024 {
            return false;
        }
        let prefix = &self.input.text[..end];
        let lexed = iris_lexer::lex(prefix.as_bytes());
        lexed.is_clean() && lexical_context(prefix, lexed.tokens(), byte)
    }
}

fn lexical_context(text: &str, tokens: &[Token], byte: usize) -> bool {
    let relevant = &tokens[..tokens.partition_point(|token| token.offset.0 <= byte)];
    let Some(last) = relevant.last() else {
        return text[..byte].trim().is_empty();
    };
    if last.end.0 < byte && !text[last.end.0..byte].trim().is_empty() {
        return false;
    }
    let current = last.offset.0 <= byte && byte <= last.end.0;
    if byte == last.end.0
        && matches!(
            last.kind,
            TokenKind::LeftBrace
                | TokenKind::RightBrace
                | TokenKind::Semicolon
                | TokenKind::Newline
        )
    {
        return true;
    }
    if current {
        if !matches!(
            last.kind,
            TokenKind::Identifier | TokenKind::Keyword | TokenKind::Newline
        ) {
            return false;
        }
        if matches!(&text[last.offset.0..last.end.0], "true" | "false" | "nil") {
            return false;
        }
    }
    let previous = if current {
        relevant.iter().rev().nth(1)
    } else {
        relevant.last()
    };
    !previous.is_some_and(|token| {
        matches!(
            token.kind,
            TokenKind::Dot
                | TokenKind::ContractView
                | TokenKind::Colon
                | TokenKind::At
                | TokenKind::DoubleAt
        )
    })
}
