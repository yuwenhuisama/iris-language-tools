use iris_analysis::HoverInfo;
use lsp_types::{ClientCapabilities, MarkupContent, MarkupKind};

#[cfg(test)]
#[path = "hover_tests.rs"]
mod tests;

const MAX_BYTES: usize = 8 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Format {
    Markdown,
    #[default]
    Plaintext,
}

impl Format {
    pub fn negotiate(capabilities: &ClientCapabilities) -> Self {
        capabilities
            .text_document
            .as_ref()
            .and_then(|text| text.hover.as_ref())
            .and_then(|hover| hover.content_format.as_ref())
            .and_then(|formats| formats.first())
            .map(|format| match format {
                MarkupKind::Markdown => Self::Markdown,
                MarkupKind::PlainText => Self::Plaintext,
            })
            .unwrap_or_default()
    }
}

pub fn render(info: &HoverInfo, format: Format) -> MarkupContent {
    let label = info
        .type_label
        .as_deref()
        .filter(|label| !label.is_empty() && !info.signature.contains(label));
    let parts = [
        info.signature.as_str(),
        label.map_or("", |_| "\n\nType: "),
        label.unwrap_or(""),
    ];
    let mut value = String::with_capacity(MAX_BYTES);
    for scalar in parts.into_iter().flat_map(str::chars) {
        let escape = match format {
            Format::Markdown => scalar.is_ascii_punctuation(),
            Format::Plaintext => false,
        };
        let extra = usize::from(escape);
        if value.len() + scalar.len_utf8() + extra > MAX_BYTES - 3 {
            value.push_str("...");
            break;
        }
        if escape {
            value.push('\\');
        }
        value.push(scalar);
    }
    MarkupContent {
        kind: match format {
            Format::Markdown => MarkupKind::Markdown,
            Format::Plaintext => MarkupKind::PlainText,
        },
        value,
    }
}
