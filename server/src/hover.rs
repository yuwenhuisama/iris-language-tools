use iris_analysis::{DocumentationInfo, HoverDetail, HoverInfo, HoverKind, ParameterCategory};
use lsp_types::{ClientCapabilities, MarkupContent, MarkupKind};

#[path = "markup.rs"]
mod markup;
use markup::Card;

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
    let mut card = Card::new(format);
    card.signature(&info.signature);
    if info.details.is_empty()
        && let Some(label) = info.type_label.as_deref()
        && !label.is_empty()
        && !info.signature.contains(label)
    {
        card.field("Type", label);
    }
    card.field("Kind", kind_label(info.kind));
    if let Some(owner) = &info.owner {
        card.field("Owner", owner);
    }
    for detail in &info.details {
        match detail {
            HoverDetail::ReturnType(label) => card.field("Declared return", label),
            HoverDetail::ValueType(label) => card.field("Type", label),
            HoverDetail::ParameterCategory(category) => card.field(
                "Parameter",
                match category {
                    ParameterCategory::Positional => "Positional",
                    ParameterCategory::Rest => "Rest",
                    ParameterCategory::Keyword => "Keyword",
                    ParameterCategory::KeywordRest => "Keyword rest",
                    ParameterCategory::Block => "Block",
                },
            ),
        }
    }
    if let Some(docs) = &info.docs {
        card.documentation(docs);
    }
    card.finish()
}

pub fn render_docs(docs: &DocumentationInfo, format: Format) -> MarkupContent {
    let mut card = Card::new(format);
    card.literal(&docs.text, docs.truncated);
    card.finish()
}

const fn kind_label(kind: HoverKind) -> &'static str {
    match kind {
        HoverKind::Class => "Class",
        HoverKind::Module => "Module",
        HoverKind::Contract => "Contract",
        HoverKind::TypeAlias => "Type alias",
        HoverKind::Binding => "Variable",
        HoverKind::Constant => "Constant",
        HoverKind::Global => "Global",
        HoverKind::Shared => "Shared",
        HoverKind::Property => "Property",
        HoverKind::Method => "Method",
        HoverKind::Parameter => "Parameter",
        HoverKind::TypeParameter => "Type parameter",
        HoverKind::PatternBinding => "Pattern binding",
    }
}
