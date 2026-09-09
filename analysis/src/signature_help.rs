use crate::{AnalysisSnapshot, FileId, SignatureHelpInfo, SignatureParameterInfo, index::Key};
use iris_builtins::{CallShape, ParameterKind};
use iris_parser::source::{ArgumentKind, CallSite, ExpressionFact, SourceKind};
use iris_syntax::ParameterCategory;

enum ActiveParameter {
    Unmapped,
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
        let key = Key { file, node: callee };
        let signatures = if let Some(target) = self.expression_symbol(key, 0) {
            let mut signature = self.signature_info(target)?;
            signature.active_parameter = match active_parameter(call, byte, &signature.parameters)?
            {
                ActiveParameter::Unmapped => None,
                ActiveParameter::Mapped(index) => Some(index),
            };
            vec![signature]
        } else {
            let member = self.builtin_member(key, 0)?;
            let separate_block = call
                .arguments
                .iter()
                .any(|argument| matches!(argument.kind, ArgumentKind::TrailingBlock))
                && member.shapes.iter().any(|shape| {
                    shape
                        .parameters
                        .iter()
                        .any(|parameter| parameter.kind == ParameterKind::Block)
                });
            member
                .shapes
                .iter()
                .filter(|shape| {
                    !separate_block
                        || shape
                            .parameters
                            .iter()
                            .any(|parameter| parameter.kind == ParameterKind::Block)
                })
                .filter_map(|shape| {
                    let active = builtin_parameter(call, byte, shape)?;
                    let mut signature = crate::builtins::presentation::signature(member, shape)?;
                    signature.active_parameter = match active {
                        ActiveParameter::Unmapped => None,
                        ActiveParameter::Mapped(index) => Some(index),
                    };
                    Some(signature)
                })
                .collect()
        };
        (!signatures.is_empty()).then_some(SignatureHelpInfo {
            signatures,
            active_signature: 0,
        })
    }
}

fn builtin_parameter(call: &CallSite, byte: usize, shape: &CallShape) -> Option<ActiveParameter> {
    let active = call.commas.iter().filter(|comma| comma.end <= byte).count();
    let mut supplied = vec![false; shape.parameters.len()];
    let mut highlight = None;
    let mut pending = false;
    for (slot, argument) in call.arguments.iter().enumerate() {
        if argument.expression.is_none()
            && argument.span.start == argument.span.end
            && matches!(argument.kind, ArgumentKind::Positional)
        {
            pending = slot > 0;
            continue;
        }
        let position = || {
            shape
                .parameters
                .iter()
                .enumerate()
                .position(|(index, parameter)| match parameter.kind {
                    ParameterKind::Positional => !supplied[index],
                    ParameterKind::Rest => true,
                    ParameterKind::Keyword | ParameterKind::Block => false,
                })
        };
        let mapped = match &argument.kind {
            ArgumentKind::Positional => position(),
            ArgumentKind::Keyword(name) => shape.parameters.iter().position(|parameter| {
                parameter.kind == ParameterKind::Keyword && parameter.label == name.text
            }),
            ArgumentKind::TrailingBlock => shape
                .parameters
                .iter()
                .position(|parameter| parameter.kind == ParameterKind::Block)
                .or_else(position),
        }?;
        if supplied[mapped] && shape.parameters[mapped].kind != ParameterKind::Rest {
            return None;
        }
        supplied[mapped] = true;
        if slot == active && !matches!(argument.kind, ArgumentKind::TrailingBlock) {
            highlight = Some(mapped);
        }
    }
    let editing_next = active > 0 && call.arguments.get(active).is_none();
    if !call.incomplete
        && !editing_next
        && shape
            .parameters
            .iter()
            .enumerate()
            .any(|(index, parameter)| !parameter.optional && !supplied[index])
    {
        return None;
    }
    let positional = shape
        .parameters
        .iter()
        .enumerate()
        .find_map(|(index, parameter)| match parameter.kind {
            ParameterKind::Positional if !supplied[index] => Some(index),
            ParameterKind::Rest => Some(index),
            ParameterKind::Positional | ParameterKind::Keyword | ParameterKind::Block => None,
        });
    let mut available = shape
        .parameters
        .iter()
        .enumerate()
        .filter_map(|(index, parameter)| {
            (Some(index) == positional
                || (parameter.kind == ParameterKind::Keyword && !supplied[index]))
                .then_some(index)
        });
    let first = available.next();
    if pending && first.is_none() {
        return None;
    }
    Some(
        highlight
            .or_else(|| first.filter(|_| available.next().is_none()))
            .map_or(ActiveParameter::Unmapped, ActiveParameter::Mapped),
    )
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
        return empty.then_some(ActiveParameter::Unmapped);
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
