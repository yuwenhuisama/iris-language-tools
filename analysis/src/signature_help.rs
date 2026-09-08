use crate::{AnalysisSnapshot, FileId, SignatureHelpInfo, SignatureParameterInfo, index::Key};
use iris_parser::source::{ArgumentKind, CallSite, ExpressionFact, SourceKind};
use iris_syntax::ParameterCategory;

enum ActiveParameter {
    Empty,
    Mapped(usize),
}

impl AnalysisSnapshot {
    #[must_use]
    pub fn signature_help(&self, file: FileId, byte: usize) -> Option<SignatureHelpInfo> {
        let cursor = self.cursor(file, byte)?;
        let document = &self.documents[&file];
        let call = document
            .source
            .calls
            .iter()
            .filter(|call| {
                call.open.end <= byte && byte <= call.close.map_or(call.end, |close| close.start)
            })
            .max_by_key(|call| call.open.start)?;
        let SourceKind::Expression(ExpressionFact::Call { callee, .. }) =
            document.source.node(call.call).kind
        else {
            return None;
        };
        let callee_node = document.source.node(callee);
        if !document.safe_scope(callee_node.scope, call.open.start)
            || (!call.incomplete && !document.safe_scope(cursor.scope, byte))
        {
            return None;
        }
        let target = self.expression_symbol(Key { file, node: callee }, 0)?;
        let signature = self.signature_info(target)?;
        let active_parameter = match active_parameter(call, byte, &signature.parameters)? {
            ActiveParameter::Empty => None,
            ActiveParameter::Mapped(index) => Some(index),
        };
        Some(SignatureHelpInfo {
            signature,
            active_parameter,
        })
    }
}

fn active_parameter(
    call: &CallSite,
    byte: usize,
    parameters: &[SignatureParameterInfo],
) -> Option<ActiveParameter> {
    let active = call.commas.iter().filter(|comma| comma.end <= byte).count();
    if parameters.is_empty() {
        let empty = call.arguments.is_empty()
            || (call.commas.is_empty()
                && call.arguments.len() == 1
                && call.arguments[0].span.start == call.arguments[0].span.end
                && call.arguments[0].expression.is_none()
                && matches!(call.arguments[0].kind, ArgumentKind::Positional));
        return empty.then_some(ActiveParameter::Empty);
    }
    let mut positional = 0;
    for index in 0..=active {
        let argument = call.arguments.get(index);
        let mapped = match argument.map(|argument| &argument.kind) {
            Some(ArgumentKind::Keyword(name)) => parameters
                .iter()
                .position(|parameter| {
                    parameter.category == ParameterCategory::Keyword && parameter.name == name.text
                })
                .or_else(|| {
                    parameters
                        .iter()
                        .position(|parameter| parameter.category == ParameterCategory::KeywordRest)
                }),
            Some(ArgumentKind::Positional | ArgumentKind::TrailingBlock) | None => {
                let target = parameters
                    .iter()
                    .enumerate()
                    .filter(|(_, parameter)| {
                        matches!(
                            parameter.category,
                            ParameterCategory::Positional | ParameterCategory::Rest
                        )
                    })
                    .find(|(index, parameter)| {
                        *index >= positional || parameter.category == ParameterCategory::Rest
                    });
                target.map(|(index, parameter)| {
                    positional = index + usize::from(parameter.category != ParameterCategory::Rest);
                    index
                })
            }
        }?;
        if index == active {
            return Some(ActiveParameter::Mapped(mapped));
        }
    }
    None
}
