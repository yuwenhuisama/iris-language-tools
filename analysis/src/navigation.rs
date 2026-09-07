use crate::{AnalysisSnapshot, FileId, Location, Span, Target, index::Key};
use iris_parser::source::{ExpressionFact, NameSite, SourceKind};

impl AnalysisSnapshot {
    #[must_use]
    pub fn definitions(&self, file: FileId, byte: usize) -> Vec<Target> {
        self.symbol_at(file, byte)
            .and_then(|key| self.symbol(key))
            .map(|symbol| Target {
                file: symbol.key.file,
                span: symbol.span,
                name_span: symbol.declaration.name.span,
            })
            .into_iter()
            .collect()
    }

    #[must_use]
    pub fn references(
        &self,
        file: FileId,
        byte: usize,
        include_declaration: bool,
    ) -> Vec<Location> {
        let Some(target) = self.symbol_at(file, byte) else {
            return Vec::new();
        };
        let mut sites = Vec::new();
        for source_file in self.documents.keys() {
            for (span, key, declaration) in self.occurrences(*source_file) {
                if key == target && (include_declaration || !declaration) {
                    sites.push(Location {
                        file: *source_file,
                        span,
                    });
                }
            }
        }
        sites.sort_by_key(|site| (site.file, site.span.start, site.span.end));
        sites.dedup();
        sites
    }

    fn symbol_at(&self, file: FileId, byte: usize) -> Option<Key> {
        let document = self.documents.get(&file)?;
        if document.protected(byte) {
            return None;
        }
        let sites = self.occurrences(file);
        sites
            .iter()
            .find(|(span, _, _)| span.start <= byte && byte < span.end)
            .or_else(|| sites.iter().find(|(span, _, _)| span.end == byte))
            .map(|(_, key, _)| *key)
    }

    fn occurrences(&self, file: FileId) -> Vec<(Span, Key, bool)> {
        let document = &self.documents[&file];
        let mut sites = Vec::new();
        for node in &document.source.nodes {
            let key = Key {
                file,
                node: node.id,
            };
            let cursor = self.node_cursor(key);
            match &node.kind {
                SourceKind::Declaration(_) => {
                    if let Some(symbol) = self.symbol(key)
                        && let Some(target) = self.declaration_key(symbol)
                    {
                        sites.push((symbol.declaration.name.span, target, true));
                    }
                }
                SourceKind::Expression(
                    ExpressionFact::Name { path } | ExpressionFact::Construction { path, .. },
                ) => {
                    for (index, site) in path.iter().enumerate() {
                        if let Some(target) = self.path(cursor, &path[..=index]) {
                            sites.push((site.span, target, false));
                        }
                    }
                    if path.len() == 1
                        && let Some(target) = self.expression_symbol(key, 0)
                    {
                        sites.push((path[0].span, target, false));
                    }
                }
                SourceKind::Expression(ExpressionFact::Member { name, .. }) => {
                    if let Some(target) = self.expression_symbol(key, 0) {
                        sites.push((name.span, target, false));
                    }
                }
                SourceKind::Type(_) => {
                    let names: Vec<NameSite> = node
                        .children
                        .iter()
                        .filter_map(|child| match &document.source.node(*child).kind {
                            SourceKind::Name(site) => Some(site.clone()),
                            _ => None,
                        })
                        .collect();
                    for (index, site) in names.iter().enumerate() {
                        let start = (0..index)
                            .rev()
                            .find(|previous| {
                                document.input.text
                                    [names[*previous].span.end..names[*previous + 1].span.start]
                                    .trim()
                                    != "::"
                            })
                            .map_or(0, |previous| previous + 1);
                        if let Some(target) = self.path(cursor, &names[start..=index]) {
                            sites.push((site.span, target, false));
                        }
                    }
                }
                SourceKind::Import(import) => {
                    if import.separators.iter().any(|separator| {
                        separator.kind == iris_parser::source::ImportSeparatorKind::Dot
                    }) {
                        continue;
                    }
                    for (index, site) in import.target.iter().enumerate() {
                        if let Some(target) = self.import_target(cursor, &import.target[..=index]) {
                            sites.push((site.span, target, false));
                        }
                    }
                    if let Some(owner) = self.import_target(cursor, &import.target) {
                        if let Some(alias) = &import.alias {
                            sites.push((alias.span, owner, false));
                        }
                        for spec in &import.specs {
                            if let Some(target) =
                                self.namespace_child(cursor, owner, &spec.name.text)
                            {
                                sites.push((spec.name.span, target, false));
                                if let Some(alias) = &spec.alias {
                                    sites.push((alias.span, target, false));
                                }
                            }
                        }
                    }
                }
                SourceKind::Expression(_)
                | SourceKind::Name(_)
                | SourceKind::Export
                | SourceKind::Body
                | SourceKind::Statement
                | SourceKind::Pattern
                | SourceKind::Decorator => {}
            }
        }
        sites.sort_by_key(|(span, _, declaration)| (span.start, span.end, *declaration));
        sites.dedup();
        sites
    }
}
