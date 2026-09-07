use crate::{
    AnalysisSnapshot,
    index::{Cursor, Key},
};
use iris_parser::source::{ExpressionFact, SourceKind};

impl AnalysisSnapshot {
    pub(crate) fn qualified_completions(&self, cursor: Cursor) -> Option<Vec<Key>> {
        let document = &self.documents[&cursor.file];
        for node in &document.source.nodes {
            let names = match &node.kind {
                SourceKind::Expression(
                    ExpressionFact::Name { path } | ExpressionFact::Construction { path, .. },
                ) => path.clone(),
                SourceKind::Type(_) => node
                    .children
                    .iter()
                    .filter_map(|child| match &document.source.node(*child).kind {
                        SourceKind::Name(site) => Some(site.clone()),
                        _ => None,
                    })
                    .collect(),
                _ => continue,
            };
            let Some(index) = names.iter().position(|site| {
                site.span.start <= cursor.offset && cursor.offset <= site.span.end
            }) else {
                continue;
            };
            if index == 0 {
                continue;
            }
            let prefix = &names[..index];
            if !names[..=index].windows(2).all(|pair| {
                document.input.text[pair[0].span.end..pair[1].span.start].trim() == "::"
            }) {
                continue;
            }
            let Some(owner) = self.path(cursor, prefix) else {
                return Some(Vec::new());
            };
            let Some(path) = self
                .symbol(owner)
                .and_then(|symbol| symbol.qualified.as_ref())
            else {
                return Some(Vec::new());
            };
            return Some(
                self.symbols
                    .iter()
                    .filter(|symbol| {
                        symbol.qualified.as_ref().is_some_and(|candidate| {
                            candidate.len() == path.len() + 1 && candidate.starts_with(path)
                        }) && self.namespace_child(cursor, owner, &symbol.declaration.name.text)
                            == Some(symbol.key)
                    })
                    .map(|symbol| symbol.key)
                    .collect(),
            );
        }
        let before = &document.input.text[..cursor.offset];
        if before.trim_end().ends_with("::") {
            return Some(Vec::new());
        }
        None
    }
}
