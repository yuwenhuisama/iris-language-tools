use iris_analysis::SignatureHelpInfo;
use lsp_types::{
    ClientCapabilities, Documentation, MarkupKind, ParameterInformation, ParameterLabel,
    SignatureHelp, SignatureInformation,
};

use crate::hover::{self, Format};

#[cfg(test)]
#[path = "signature_help_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Options {
    pub format: Format,
    pub label_offsets: bool,
}

impl Options {
    pub fn negotiate(capabilities: &ClientCapabilities) -> Self {
        let settings = capabilities
            .text_document
            .as_ref()
            .and_then(|text| text.signature_help.as_ref())
            .and_then(|help| help.signature_information.as_ref());
        Self {
            format: settings
                .and_then(|settings| settings.documentation_format.as_ref())
                .and_then(|formats| formats.first())
                .map(|format| match format {
                    MarkupKind::Markdown => Format::Markdown,
                    MarkupKind::PlainText => Format::Plaintext,
                })
                .unwrap_or_default(),
            label_offsets: settings
                .and_then(|settings| settings.parameter_information.as_ref())
                .is_some_and(|parameters| parameters.label_offset_support == Some(true)),
        }
    }
}

pub fn render(info: SignatureHelpInfo, options: Options) -> Option<SignatureHelp> {
    let signature = info.signatures.get(info.active_signature)?;
    let active_parameter = match signature.active_parameter {
        Some(index) => {
            signature.parameters.get(index)?;
            Some(u32::try_from(index).ok()?)
        }
        None if signature.parameters.is_empty() => None,
        None => return None,
    };
    let multiple = info.signatures.len() > 1;
    let signatures = info
        .signatures
        .into_iter()
        .map(|signature| {
            if signature.active_parameter.is_none() && !signature.parameters.is_empty() {
                return None;
            }
            let parameters = signature
                .parameters
                .iter()
                .map(|parameter| {
                    let span = parameter.label;
                    let text = signature.label.get(span.start..span.end)?;
                    let label = if options.label_offsets {
                        let start = signature.label.get(..span.start)?.encode_utf16().count();
                        let end = start.checked_add(text.encode_utf16().count())?;
                        ParameterLabel::LabelOffsets([
                            u32::try_from(start).ok()?,
                            u32::try_from(end).ok()?,
                        ])
                    } else {
                        ParameterLabel::Simple(text.into())
                    };
                    Some(ParameterInformation {
                        label,
                        documentation: parameter.docs.as_ref().map(|docs| {
                            Documentation::MarkupContent(hover::render_docs(docs, options.format))
                        }),
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Some(SignatureInformation {
                label: signature.label,
                documentation: signature.docs.as_ref().map(|docs| {
                    Documentation::MarkupContent(hover::render_docs(docs, options.format))
                }),
                parameters: Some(parameters),
                active_parameter: match signature.active_parameter.filter(|_| multiple) {
                    Some(index) => {
                        signature.parameters.get(index)?;
                        Some(u32::try_from(index).ok()?)
                    }
                    None => None,
                },
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(SignatureHelp {
        signatures,
        active_signature: Some(u32::try_from(info.active_signature).ok()?),
        active_parameter,
    })
}
