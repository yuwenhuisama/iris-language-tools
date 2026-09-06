use iris_lexer::ByteOffset;
use lsp_types::Position;

pub fn position(text: &str, offset: ByteOffset) -> Option<Position> {
    text.get(..offset.0)?;
    let mut characters = text.char_indices().peekable();
    let mut line = 0_u32;
    let mut column = 0_u32;
    while let Some((index, scalar)) = characters.next() {
        if index >= offset.0 {
            break;
        }
        match scalar {
            '\r' => {
                if let Some(&(next, '\n')) = characters.peek() {
                    if next == offset.0 {
                        break;
                    }
                    characters.next();
                }
                line = line.checked_add(1)?;
                column = 0;
            }
            '\n' => {
                line = line.checked_add(1)?;
                column = 0;
            }
            _ => column = column.checked_add(u32::try_from(scalar.len_utf16()).ok()?)?,
        }
    }
    Some(Position::new(line, column))
}

#[cfg(test)]
mod tests {
    use super::position;
    use iris_lexer::ByteOffset;
    use lsp_types::Position;

    #[test]
    fn maps_utf16_when_source_contains_bom_and_astral_scalars() {
        let text = "\u{feff}a\u{1f600}z";
        let actual = position(text, ByteOffset(8));
        assert_eq!(actual, Some(Position::new(0, 4)));
    }

    #[test]
    fn maps_physical_lines_when_source_mixes_newlines() {
        let text = "a\r\nb\rc\nd";
        let actual: Vec<_> = (0..=text.len())
            .map(|offset| position(text, ByteOffset(offset)))
            .collect();
        assert_eq!(
            actual,
            [
                (0, 0),
                (0, 1),
                (0, 1),
                (1, 0),
                (1, 1),
                (2, 0),
                (2, 1),
                (3, 0),
                (3, 1)
            ]
            .map(|(line, column)| Some(Position::new(line, column)))
        );
    }

    #[test]
    fn rejects_offsets_when_outside_scalar_boundaries() {
        let text = "\u{1f600}";
        let actual = [1, 2, 3, 5].map(|offset| position(text, ByteOffset(offset)));
        assert_eq!(actual, [None; 4]);
    }
}
