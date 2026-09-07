use super::{LineIndex, position};
use iris_lexer::ByteOffset;
use lsp_types::Position;

#[test]
fn roundtrips_boundaries_when_text_has_bom_astral_and_mixed_lines() {
    let text = "\u{feff}a\u{1f600}z\r\nb\rc\n";
    let index = LineIndex::new(text);

    for offset in (0..=text.len()).filter(|&offset| text.is_char_boundary(offset)) {
        let actual = index.position(ByteOffset(offset));

        assert_eq!(actual, position(text, ByteOffset(offset)));
        let canonical = if text.as_bytes().get(offset) == Some(&b'\n')
            && offset > 0
            && text.as_bytes()[offset - 1] == b'\r'
        {
            offset - 1
        } else {
            offset
        };
        assert_eq!(
            index.byte_offset(actual.unwrap()),
            Some(ByteOffset(canonical))
        );
    }
}

#[test]
fn clamps_columns_when_position_is_past_eol() {
    let index = LineIndex::new("a\r\n\u{1f600}\r");

    let actual = [
        Position::new(0, 99),
        Position::new(1, 99),
        Position::new(2, 99),
    ]
    .map(|position| index.byte_offset(position));

    assert_eq!(
        actual,
        [
            Some(ByteOffset(1)),
            Some(ByteOffset(7)),
            Some(ByteOffset(8))
        ]
    );
}

#[test]
fn rejects_coordinates_when_line_or_scalar_boundary_is_invalid() {
    let index = LineIndex::new("\u{1f600}");

    let actual =
        [Position::new(0, 1), Position::new(1, 0)].map(|position| index.byte_offset(position));

    assert_eq!(actual, [None, None]);
    assert_eq!(index.position(ByteOffset(1)), None);
    assert_eq!(index.position(ByteOffset(5)), None);
}

#[test]
fn maps_eof_when_document_is_empty() {
    let index = LineIndex::new("");

    let actual = index.byte_offset(Position::new(0, u32::MAX));

    assert_eq!(actual, Some(ByteOffset(0)));
    assert_eq!(index.position(ByteOffset(0)), Some(Position::new(0, 0)));
}
