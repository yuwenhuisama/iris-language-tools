use iris_lexer::{Token, TokenKind};

use crate::SkipReason;

const INPUT_LIMIT: usize = 256 * 1024;
pub const OUTPUT_LIMIT: usize = 1024 * 1024;
const NESTING_LIMIT: usize = 128;

pub fn check_source(source: &str) -> Result<(), SkipReason> {
    if source.len() > INPUT_LIMIT {
        return Err(SkipReason::InputLimit);
    }
    let bytes = source.as_bytes();
    if bytes
        .windows(2)
        .any(|pair| matches!(pair, [b'\\', b'\r' | b'\n']))
    {
        return Err(SkipReason::Continuation);
    }
    if source.contains("\"\"\"") || source.contains("'''") || source.contains("${") {
        return Err(SkipReason::UnsupportedLiteral);
    }
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'r' {
            let following = bytes[index + 1..].iter().find(|byte| **byte != b'#');
            if matches!(following, Some(b'\'' | b'"')) {
                return Err(SkipReason::UnsupportedLiteral);
            }
        }
    }
    Ok(())
}

pub struct Delimiters {
    closers: [u8; NESTING_LIMIT],
    depth: usize,
}

impl Delimiters {
    pub const fn new() -> Self {
        Self {
            closers: [0; NESTING_LIMIT],
            depth: 0,
        }
    }

    pub const fn depth(&self) -> usize {
        self.depth
    }

    pub fn consume(&mut self, token: Token, source: &str) -> Result<(), SkipReason> {
        let opener = match token.kind {
            TokenKind::LeftBrace | TokenKind::HashOpen => Some(b'}'),
            TokenKind::LeftParen => Some(b')'),
            TokenKind::SourceCharacter if source.as_bytes().get(token.offset.0) == Some(&b'[') => {
                Some(b']')
            }
            _ => None,
        };
        if let Some(closer) = opener {
            let Some(slot) = self.closers.get_mut(self.depth) else {
                return Err(SkipReason::NestingLimit);
            };
            *slot = closer;
            self.depth += 1;
        } else if let Some(closer) = closing(token, source) {
            let Some(depth) = self.depth.checked_sub(1) else {
                return Err(SkipReason::MismatchedDelimiter);
            };
            if self.closers[depth] != closer {
                return Err(SkipReason::MismatchedDelimiter);
            }
            self.depth = depth;
        }
        Ok(())
    }
}

pub fn closing(token: Token, source: &str) -> Option<u8> {
    match token.kind {
        TokenKind::RightBrace => Some(b'}'),
        TokenKind::RightParen => Some(b')'),
        TokenKind::SourceCharacter if source.as_bytes().get(token.offset.0) == Some(&b']') => {
            Some(b']')
        }
        _ => None,
    }
}
