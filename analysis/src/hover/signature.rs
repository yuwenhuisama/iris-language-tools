use super::text::BoundedText;
use crate::{AnalysisSnapshot, Document, Span, index::Symbol};
use iris_lexer::TokenKind;
use iris_parser::source::{DeclarationKind, ScopeKind, SourceKind, SyntaxNode};
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
            DeclarationKind::Method => document.hover_method(symbol, output)?,
            DeclarationKind::Parameter => {
                let dynamic = document.source.scope(symbol.scope).kind == ScopeKind::Method;
                document.hover_parameter(document.source.node(symbol.key.node), output, dynamic);
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
    fn hover_method(&self, symbol: &Symbol, output: &mut BoundedText) -> Option<()> {
        let declaration = &symbol.declaration;
        let close = declaration.return_hint_offset?;
        let end = declaration
            .return_type
            .map_or(close, |id| self.source.node(id).span.end);
        if self
            .source
            .recovery
            .iter()
            .any(|region| region.span.start < end && region.span.end > symbol.span.start)
        {
            return None;
        }
        let prefix = Span {
            start: symbol.span.start,
            end: declaration.name.span.start,
        };
        let explicit_visibility = self.source.tokens.iter().any(|token| {
            token.offset.0 >= prefix.start
                && token.end.0 <= prefix.end
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
        let mut start = symbol.span.start;
        for child in &self.source.node(symbol.key.node).children {
            let node = self.source.node(*child);
            if node.span.end > close {
                continue;
            }
            let parameter = match &node.kind {
                SourceKind::Declaration(value) => value.kind == DeclarationKind::Parameter,
                SourceKind::Statement => node.children.iter().any(|id| {
                    matches!(&self.source.node(*id).kind, SourceKind::Name(site) if site.text == "_")
                }),
                _ => false,
            };
            if parameter {
                self.hover_source(
                    Span {
                        start,
                        end: node.span.start,
                    },
                    output,
                );
                if self.input.text[start..node.span.start].ends_with(char::is_whitespace) {
                    output.push(" ");
                }
                self.hover_parameter(node, output, true);
                start = node.span.end;
            }
            if output.full() {
                break;
            }
        }
        self.hover_source(Span { start, end }, output);
        if declaration.return_type.is_none() {
            output.push(" -> Dynamic<Object>");
        }
        Some(())
    }

    fn hover_parameter(&self, node: &SyntaxNode, output: &mut BoundedText, dynamic: bool) {
        let annotation = node
            .children
            .iter()
            .any(|id| matches!(self.source.node(*id).kind, SourceKind::Type(_)));
        let name = node
            .children
            .iter()
            .find_map(|id| match &self.source.node(*id).kind {
                SourceKind::Name(site) => Some(site),
                _ => None,
            });
        if dynamic
            && !annotation
            && let Some(name) = name
        {
            self.hover_source(
                Span {
                    start: node.span.start,
                    end: name.span.end,
                },
                output,
            );
            output.push(": Dynamic<Object>");
            if self.input.text[name.span.end..node.span.end].starts_with('=') {
                output.push(" ");
            }
            self.hover_source(
                Span {
                    start: name.span.end,
                    end: node.span.end,
                },
                output,
            );
        } else {
            self.hover_source(node.span, output);
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
