use iris_analysis::DocumentationInfo;
use lsp_types::{MarkupContent, MarkupKind};

use super::{Format, MAX_BYTES};

pub(super) struct Card {
    value: String,
    format: Format,
    full: bool,
}

impl Card {
    pub(super) fn new(format: Format) -> Self {
        Self {
            value: String::with_capacity(MAX_BYTES),
            format,
            full: false,
        }
    }

    pub(super) fn signature(&mut self, signature: &str) {
        match self.format {
            Format::Plaintext => self.literal(signature, false),
            Format::Markdown => {
                let mut end = 0;
                let mut run = 0;
                let mut longest = 0;
                for scalar in signature.chars() {
                    run = if scalar == '`' { run + 1 } else { 0 };
                    let candidate = longest.max(run);
                    let length = end + scalar.len_utf8();
                    if length + 2 * (candidate + 1).max(3) + 9 > MAX_BYTES / 2 {
                        break;
                    }
                    longest = candidate;
                    end = length;
                }
                let fence = "`".repeat((longest + 1).max(3));
                self.value.push_str(&fence);
                self.value.push_str("iris\n");
                self.value.push_str(&signature[..end]);
                if end < signature.len() {
                    self.value.push_str("...");
                }
                self.value.push('\n');
                self.value.push_str(&fence);
            }
        }
    }

    pub(super) fn literal(&mut self, text: &str, truncated: bool) {
        if self.full {
            return;
        }
        for scalar in text.chars() {
            let entity = match (self.format, scalar) {
                (Format::Markdown, '<') => Some("&lt;"),
                (Format::Markdown, '>') => Some("&gt;"),
                (Format::Markdown, '&') => Some("&amp;"),
                _ => None,
            };
            let escape = match self.format {
                Format::Markdown => scalar.is_ascii_punctuation() && entity.is_none(),
                Format::Plaintext => false,
            };
            let length = entity.map_or_else(|| scalar.len_utf8() + usize::from(escape), str::len);
            if self.value.len() + length > MAX_BYTES - 3 {
                self.value.push_str("...");
                self.full = true;
                return;
            }
            if let Some(entity) = entity {
                self.value.push_str(entity);
                continue;
            }
            if escape {
                self.value.push('\\');
            }
            self.value.push(scalar);
        }
        if truncated {
            self.value.push_str("...");
        }
    }

    pub(super) fn field(&mut self, label: &str, text: &str) {
        let prefix = match self.format {
            Format::Markdown => format!("\n\n**{label}:** "),
            Format::Plaintext => format!("\n\n{label}: "),
        };
        if self.section(&prefix) {
            self.literal(text, false);
        }
    }

    pub(super) fn documentation(&mut self, docs: &DocumentationInfo) {
        let prefix = match self.format {
            Format::Markdown => "\n\n---\n\n**Documentation**\n\n",
            Format::Plaintext => "\n\nDocumentation\n\n",
        };
        if self.section(prefix) {
            self.literal(&docs.text, docs.truncated);
        }
    }

    fn section(&mut self, prefix: &str) -> bool {
        if self.full {
            return false;
        }
        if self.value.len() + prefix.len() + 3 > MAX_BYTES {
            self.value.push_str("...");
            self.full = true;
            return false;
        }
        self.value.push_str(prefix);
        true
    }

    pub(super) fn finish(self) -> MarkupContent {
        MarkupContent {
            kind: match self.format {
                Format::Markdown => MarkupKind::Markdown,
                Format::Plaintext => MarkupKind::PlainText,
            },
            value: self.value,
        }
    }
}
