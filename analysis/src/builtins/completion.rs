use crate::{
    AnalysisSnapshot, CompletionItem, CompletionKind, Span,
    index::{Cursor, Key},
};
use iris_builtins::Surface;
use iris_parser::source::{ExpressionFact, SourceKind};

impl AnalysisSnapshot {
    pub(crate) fn builtin_member_completions(
        &self,
        receiver: Key,
        replace: Span,
    ) -> Vec<CompletionItem> {
        self.builtin_receiver(receiver, 0)
            .into_iter()
            .flat_map(super::BuiltinReceiver::members)
            .filter(|member| self.builtin_unchanged(receiver.file, member))
            .filter(|member| self.builtin_selector_available(receiver, member.selector))
            .map(|member| CompletionItem {
                label: member.selector.into(),
                detail: member
                    .shapes
                    .first()
                    .and_then(|shape| super::presentation::signature(member, shape))
                    .map(|info| info.label)
                    .or_else(|| Some(format!("builtin {}.{}", member.owner, member.selector))),
                kind: if member.surface == Surface::Property {
                    CompletionKind::Property
                } else {
                    CompletionKind::Method
                },
                replace,
            })
            .collect()
    }

    pub(crate) fn builtin_name_completions(
        &self,
        cursor: Cursor,
        replace: Span,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();
        for member in iris_builtins::members()
            .iter()
            .filter(|member| member.surface == Surface::Global)
        {
            if self.builtin_name_available(cursor, member.selector) {
                items.push(CompletionItem {
                    label: member.selector.into(),
                    kind: CompletionKind::Method,
                    replace,
                    detail: member
                        .shapes
                        .first()
                        .and_then(|shape| super::presentation::signature(member, shape))
                        .map(|info| info.label),
                });
            }
        }
        for (names, kind) in [
            (iris_builtins::class_names(), CompletionKind::Class),
            (iris_builtins::service_names(), CompletionKind::Module),
            (&["Transformation"][..], CompletionKind::Constant),
        ] {
            for name in names {
                let Some(head) = name.split("::").next() else {
                    continue;
                };
                if self.builtin_name_available(cursor, head) {
                    items.push(CompletionItem {
                        label: head.into(),
                        detail: Some(format!("builtin {head}")),
                        kind,
                        replace,
                    });
                }
            }
        }
        items
    }

    pub(crate) fn builtin_qualified_completions(
        &self,
        cursor: Cursor,
        replace: Span,
    ) -> Vec<CompletionItem> {
        let document = &self.documents[&cursor.file];
        for node in &document.source.nodes {
            let SourceKind::Expression(ExpressionFact::Name { path }) = &node.kind else {
                continue;
            };
            let Some(index) = path.iter().position(|site| {
                site.span.start <= cursor.offset && cursor.offset <= site.span.end
            }) else {
                continue;
            };
            if index == 0 || !self.builtin_name_available(cursor, &path[0].text) {
                continue;
            }
            let prefix = format!(
                "{}::",
                path[..index]
                    .iter()
                    .map(|site| site.text.as_str())
                    .collect::<Vec<_>>()
                    .join("::")
            );
            return iris_builtins::service_names()
                .iter()
                .filter_map(|name| {
                    let suffix = name.strip_prefix(&prefix)?.split("::").next()?;
                    Some(CompletionItem {
                        label: suffix.into(),
                        detail: Some(format!("builtin {name}")),
                        kind: CompletionKind::Module,
                        replace,
                    })
                })
                .collect();
        }
        Vec::new()
    }
}
