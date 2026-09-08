//! Immutable, non-evaluating semantic queries over parser-owned source facts.
//!
//! Coordinates are half-open UTF-8 byte spans. File and group identity belong to
//! the host; this crate neither opens files nor discovers package dependencies.

mod binding_types;
mod completion;
mod hints;
mod hover;
mod index;
mod keyword_context;
mod members;
mod model;
mod navigation;
mod qualified;
mod resolve;
mod signature_help;
mod types;

pub use model::*;

use iris_parser::source::SourceDocument;
use std::collections::BTreeMap;

#[derive(Debug)]
struct Document {
    input: SourceInput,
    source: SourceDocument,
    parents: Vec<Option<iris_parser::source::SyntaxId>>,
}

#[derive(Debug)]
pub struct AnalysisSnapshot {
    documents: BTreeMap<FileId, Document>,
    symbols: Vec<index::Symbol>,
}

impl AnalysisSnapshot {
    /// Parse a complete host-selected snapshot. Repeated IDs use the last input.
    #[must_use]
    pub fn new(inputs: impl IntoIterator<Item = SourceInput>) -> Self {
        let documents = inputs
            .into_iter()
            .map(|input| {
                let source = iris_parser::parse_editor(&input.text).source;
                let mut parents = vec![None; source.nodes.len()];
                for node in &source.nodes {
                    for child in &node.children {
                        parents[child.0] = Some(node.id);
                    }
                }
                (
                    input.id,
                    Document {
                        input,
                        source,
                        parents,
                    },
                )
            })
            .collect();
        let mut snapshot = Self {
            documents,
            symbols: Vec::new(),
        };
        snapshot.symbols = snapshot.collect_symbols();
        snapshot
    }
}
