use iris_lexer::ByteOffset;
use lsp_types::Position;

#[derive(Debug)]
pub struct LineIndex<'text> {
    text: &'text str,
    lines: Vec<std::ops::Range<usize>>,
}

impl<'text> LineIndex<'text> {
    pub(crate) fn new(text: &'text str) -> Self {
        let mut lines = Vec::new();
        let mut start = 0;
        let mut characters = text.char_indices().peekable();
        while let Some((offset, scalar)) = characters.next() {
            if matches!(scalar, '\r' | '\n') {
                lines.push(start..offset);
                start = offset + 1;
                if scalar == '\r' && characters.peek().is_some_and(|&(_, next)| next == '\n') {
                    characters.next();
                    start += 1;
                }
            }
        }
        lines.push(start..text.len());
        Self { text, lines }
    }

    pub(crate) fn position(&self, offset: ByteOffset) -> Option<Position> {
        self.text.get(..offset.0)?;
        let line = self.lines.partition_point(|range| range.start <= offset.0) - 1;
        let range = &self.lines[line];
        let column = self.text[range.start..offset.0.min(range.end)]
            .encode_utf16()
            .count();
        Some(Position::new(
            u32::try_from(line).ok()?,
            u32::try_from(column).ok()?,
        ))
    }

    pub(crate) fn byte_offset(&self, position: Position) -> Option<ByteOffset> {
        let range = self.lines.get(usize::try_from(position.line).ok()?)?;
        let mut column = 0_u32;
        for (offset, scalar) in self.text[range.clone()].char_indices() {
            if column == position.character {
                return Some(ByteOffset(range.start + offset));
            }
            column = column.checked_add(u32::try_from(scalar.len_utf16()).ok()?)?;
            if column > position.character {
                return None;
            }
        }
        Some(ByteOffset(range.end))
    }
}

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
#[path = "workspace/position_tests.rs"]
mod index_tests;

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
