use super::text::BoundedText;
use crate::{AnalysisSnapshot, Document, SignatureParameterInfo, Span, index::Symbol};
use iris_lexer::TokenKind;
use iris_parser::source::{DeclarationKind, ParameterSlot, ScopeKind};
use iris_syntax::Visibility;

impl AnalysisSnapshot {
    pub(super) fn hover_signature(
        &self,
        symbol: &Symbol,
        output: &mut BoundedText,
        label: Option<&str>,
    ) -> Option<()> {
        let document = &self.documents[&symbol.key.file];
        let declaration = &symbol.declaration;
        match declaration.kind {
            DeclarationKind::Class | DeclarationKind::Module | DeclarationKind::Contract => {
                let header = declaration.header.as_ref()?;
                if !header.complete {
                    return None;
                }
                document.hover_source(header.span, output);
            }
            DeclarationKind::Method => {
                document.method_display(symbol, output)?;
            }
            DeclarationKind::Parameter => {
                let slot = document
                    .source
                    .parameter_slots
                    .iter()
                    .find(|slot| slot.declaration == Some(symbol.key.node))?;
                let dynamic = document.source.scope(symbol.scope).kind == ScopeKind::Method;
                document.parameter_display(slot, output, dynamic);
            }
            DeclarationKind::Binding
            | DeclarationKind::Constant
            | DeclarationKind::Global
            | DeclarationKind::Shared
            | DeclarationKind::Property
            | DeclarationKind::PatternBinding
            | DeclarationKind::TypeParameter
            | DeclarationKind::TypeAlias => {
                let end = declaration
                    .annotation
                    .map_or(declaration.name.span.end, |id| {
                        document.source.node(id).span.end
                    });
                document.hover_source(
                    Span {
                        start: symbol.span.start,
                        end,
                    },
                    output,
                );
                if declaration.annotation.is_none()
                    && let Some(label) = label
                {
                    output.push(": ");
                    output.push(label);
                }
            }
        }
        Some(())
    }
}

impl Document {
    pub(super) fn method_display(
        &self,
        symbol: &Symbol,
        output: &mut BoundedText,
    ) -> Option<Vec<SignatureParameterInfo>> {
        let declaration = &symbol.declaration;
        let signature = self
            .source
            .signatures
            .iter()
            .find(|site| site.owner == symbol.key.node)?;
        if !signature.valid {
            return None;
        }
        let explicit_visibility = self.source.tokens.iter().any(|token| {
            token.offset.0 >= signature.span.start
                && token.end.0 <= declaration.name.span.start
                && matches!(
                    &self.input.text[token.offset.0..token.end.0],
                    "public" | "private" | "protected"
                )
        });
        if !explicit_visibility {
            output.push(match declaration.visibility {
                Visibility::Public => "public ",
                Visibility::Private => "private ",
                Visibility::Protected => "protected ",
            });
        }
        let mut start = signature.span.start;
        let mut parameters = Vec::new();
        for slot in self
            .source
            .parameter_slots
            .iter()
            .filter(|slot| slot.owner == symbol.key.node)
        {
            self.hover_source(
                Span {
                    start,
                    end: slot.span.start,
                },
                output,
            );
            if self.input.text[start..slot.span.start].ends_with(char::is_whitespace) {
                output.push(" ");
            }
            let label_start = output.len();
            self.parameter_display(slot, output, true);
            if output.full() {
                return Some(parameters);
            }
            parameters.push(SignatureParameterInfo {
                label: Span {
                    start: label_start,
                    end: output.len(),
                },
                name: slot.name.text.clone(),
                category: slot.category,
                docs: slot
                    .declaration
                    .and_then(|id| {
                        self.source
                            .documentation
                            .iter()
                            .find(|docs| docs.declaration == id)
                    })
                    .map(|docs| crate::DocumentationInfo {
                        text: docs.text.clone(),
                        truncated: docs.truncated,
                    }),
            });
            start = slot.span.end;
        }
        self.hover_source(
            Span {
                start,
                end: signature.span.end,
            },
            output,
        );
        if signature.return_type.is_none() {
            output.push(" -> Dynamic<Object>");
        }
        Some(parameters)
    }

    fn parameter_display(&self, slot: &ParameterSlot, output: &mut BoundedText, dynamic: bool) {
        if dynamic && slot.annotation.is_none() {
            self.hover_source(
                Span {
                    start: slot.span.start,
                    end: slot.name.span.end,
                },
                output,
            );
            output.push(": Dynamic<Object>");
            if self.input.text[slot.name.span.end..slot.span.end].starts_with('=') {
                output.push(" ");
            }
            self.hover_source(
                Span {
                    start: slot.name.span.end,
                    end: slot.span.end,
                },
                output,
            );
        } else {
            self.hover_source(slot.span, output);
        }
    }

    pub(super) fn hover_source(&self, span: Span, output: &mut BoundedText) {
        let mut previous = span.start;
        for token in self
            .source
            .tokens
            .iter()
            .skip_while(|token| token.end.0 <= span.start)
        {
            if token.offset.0 >= span.end || output.full() {
                break;
            }
            if token.kind == TokenKind::Newline {
                continue;
            }
            if token.offset.0 > previous {
                output.push(" ");
            }
            output.push(&self.input.text[token.offset.0..token.end.0]);
            previous = token.end.0;
        }
    }
}
