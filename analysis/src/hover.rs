mod labels;
mod presentation;
mod signature;
mod text;

use crate::{AnalysisSnapshot, FileId, HoverInfo, index::Key};
use text::BoundedText;

impl AnalysisSnapshot {
    #[must_use]
    pub fn hover(&self, file: FileId, byte: usize) -> Option<HoverInfo> {
        let cursor = self.cursor(file, byte)?;
        let document = &self.documents[&file];
        if document.protected(byte) || !document.safe_scope(cursor.scope, byte) {
            return None;
        }
        let mut matches = self
            .occurrences(file)
            .into_iter()
            .filter(|(span, _, _)| span.start <= byte && byte < span.end);
        let (span, key, _) = matches.next()?;
        if matches.any(|(other_span, other_key, _)| other_span != span || other_key != key) {
            return None;
        }
        self.local_hover(key, span)
    }

    fn local_hover(&self, key: Key, span: crate::Span) -> Option<HoverInfo> {
        let symbol = self.symbol(key)?;
        let document = &self.documents[&key.file];
        if !document.safe_scope(symbol.scope, symbol.declaration.name.span.end) {
            return None;
        }
        let type_label = self.hover_label(key, 0);
        let mut signature = BoundedText::new(4096 - type_label.as_ref().map_or(0, String::len));
        self.hover_signature(symbol, &mut signature, type_label.as_deref())?;
        Some(HoverInfo {
            span,
            signature: signature.finish(),
            type_label,
            kind: symbol.declaration.kind,
            owner: self.hover_owner(key),
            details: self.hover_details(key),
            docs: self.documentation(key),
        })
    }

    pub(crate) fn signature_info(&self, key: Key) -> Option<crate::SignatureInfo> {
        let symbol = self.symbol(key)?;
        if symbol.declaration.kind != iris_parser::source::DeclarationKind::Method {
            return None;
        }
        let document = &self.documents[&key.file];
        if !document.safe_scope(symbol.scope, symbol.declaration.name.span.end) {
            return None;
        }
        let mut output = BoundedText::new(4096);
        let parameters = document.method_display(symbol, &mut output)?;
        if output.full() {
            return None;
        }
        Some(crate::SignatureInfo {
            label: output.finish(),
            parameters,
            docs: self.documentation(key),
        })
    }
}
