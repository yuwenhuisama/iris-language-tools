use super::{CoverageIssue, IssueKind, MAX_FILE_BYTES, MAX_FILES, MAX_TOTAL_BYTES};
use crate::documents::Documents;
use lsp_types::Uri;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentVersion(pub i32);

#[derive(Clone, Debug)]
pub struct DocumentGeneration(Arc<()>);

impl PartialEq for DocumentGeneration {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for DocumentGeneration {}

#[derive(Clone, Debug)]
pub(super) struct Overlay {
    pub(super) uri: Uri,
    pub(super) text: Arc<str>,
    pub(super) version: DocumentVersion,
    pub(super) generation: DocumentGeneration,
}

#[derive(Clone, Debug, Default)]
pub struct OverlaySet {
    pub(super) files: BTreeMap<String, Overlay>,
    pub(super) issues: Vec<CoverageIssue>,
    pub(super) excluded: Vec<Uri>,
}

impl OverlaySet {
    pub(crate) fn capture(documents: &Documents) -> Self {
        let mut result = Self::default();
        let mut bytes = 0;
        let mut documents: Vec<_> = documents.iter().collect();
        documents.sort_by_key(|(uri, _)| uri.as_str());
        for (uri, document) in documents {
            let issue = if document.text.len() > MAX_FILE_BYTES {
                Some(IssueKind::FileBytes)
            } else if result.files.len() >= MAX_FILES {
                Some(IssueKind::FileCount)
            } else if bytes + document.text.len() > MAX_TOTAL_BYTES {
                Some(IssueKind::TotalBytes)
            } else {
                None
            };
            if let Some(kind) = issue {
                result.issues.push(CoverageIssue {
                    uri: Some(uri.clone()),
                    path: None,
                    kind,
                });
                result.excluded.push(uri.clone());
                continue;
            }
            bytes += document.text.len();
            result.files.insert(
                uri.as_str().to_owned(),
                Overlay {
                    uri: uri.clone(),
                    text: Arc::from(document.text.as_str()),
                    version: DocumentVersion(document.version),
                    generation: DocumentGeneration(Arc::clone(&document.generation)),
                },
            );
        }
        result
    }
}
