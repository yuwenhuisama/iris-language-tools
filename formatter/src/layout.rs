use iris_lexer::{ByteOffset, Token, TokenKind};

use crate::safety::{Delimiters, OUTPUT_LIMIT, closing};
use crate::{FormatOptions, SkipReason};

pub fn indent(
    source: &str,
    tokens: &[Token],
    options: FormatOptions,
) -> Result<(String, Vec<Token>), SkipReason> {
    let unit = if options.insert_spaces {
        " ".repeat(usize::from(options.tab_size))
    } else {
        "\t".into()
    };
    let mut output = String::with_capacity(source.len());
    let mut mapped = Vec::with_capacity(tokens.len());
    let mut delimiters = Delimiters::new();
    let mut remaining = tokens;
    let mut start = 0;
    for line in source.split_inclusive(['\r', '\n']) {
        let end = start + line.len();
        let count = remaining.partition_point(|token| token.offset.0 < end);
        let (line_tokens, rest) = remaining.split_at(count);
        remaining = rest;
        let bom = if start == 0 && line.starts_with('\u{feff}') {
            3
        } else {
            0
        };
        let content = line[bom..].trim_start_matches([' ', '\t']);
        let prefix = line.len() - content.len();
        let eligible = line_tokens.first().is_some_and(|token| {
            token.kind != TokenKind::Newline && token.offset.0 == start + prefix
        });
        let depth = delimiters.depth();
        let mut leading = 0;
        let mut closer_end = start + prefix;
        for &token in line_tokens {
            if closing(token, source).is_some()
                && source[closer_end..token.offset.0]
                    .bytes()
                    .all(|byte| matches!(byte, b' ' | b'\t'))
            {
                leading += 1;
                closer_end = token.offset.0 + 1;
            } else {
                break;
            }
        }
        for &token in line_tokens {
            delimiters.consume(token, source)?;
        }
        let indentation = if eligible {
            depth
                .checked_sub(leading)
                .ok_or(SkipReason::MismatchedDelimiter)?
                * unit.len()
        } else {
            prefix - bom
        };
        let new_prefix = bom + indentation;
        if output.len() + new_prefix + content.len() > OUTPUT_LIMIT {
            return Err(SkipReason::OutputLimit);
        }
        let output_start = output.len();
        if eligible {
            output.push_str(&line[..bom]);
            for _ in 0..(indentation / unit.len()) {
                output.push_str(&unit);
            }
            output.push_str(content);
        } else {
            output.push_str(line);
        }
        for token in line_tokens {
            let relative = token
                .offset
                .0
                .checked_sub(start + prefix)
                .ok_or(SkipReason::UncertainProtectedRange)?;
            mapped.push(Token {
                kind: token.kind,
                offset: ByteOffset(output_start + new_prefix + relative),
            });
        }
        start = end;
    }
    if delimiters.depth() != 0 {
        return Err(SkipReason::UnclosedDelimiter);
    }
    Ok((output, mapped))
}
