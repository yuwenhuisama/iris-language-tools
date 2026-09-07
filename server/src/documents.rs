use std::collections::HashMap;
use std::sync::Arc;

use lsp_types::{DidChangeTextDocumentParams, TextDocumentItem, Uri};

pub struct Document {
    pub(crate) text: String,
    pub(crate) version: i32,
    pub(crate) generation: Arc<()>,
}

#[derive(Default)]
pub struct Documents(HashMap<Uri, Document>);

impl Documents {
    pub(crate) fn open(&mut self, item: TextDocumentItem) -> Option<Uri> {
        if !item
            .uri
            .scheme()
            .is_some_and(|scheme| matches!(scheme.as_str(), "file" | "untitled"))
            || self.0.contains_key(&item.uri)
        {
            return None;
        }
        let uri = item.uri;
        self.0.insert(
            uri.clone(),
            Document {
                text: item.text,
                version: item.version,
                generation: Arc::new(()),
            },
        );
        Some(uri)
    }

    pub(crate) fn change(
        &mut self,
        change: DidChangeTextDocumentParams,
    ) -> anyhow::Result<Option<Uri>> {
        let uri = change.text_document.uri;
        let Some(document) = self.0.get_mut(&uri) else {
            return Ok(None);
        };
        if change.text_document.version <= document.version {
            return Ok(None);
        }
        anyhow::ensure!(
            change
                .content_changes
                .iter()
                .all(|edit| edit.range.is_none() && edit.range_length.is_none()),
            "full synchronization requires whole-document changes"
        );
        if let Some(edit) = change.content_changes.into_iter().last() {
            document.text = edit.text;
        }
        document.version = change.text_document.version;
        Ok(Some(uri))
    }

    pub(crate) fn get(&self, uri: &Uri) -> Option<&Document> {
        self.0.get(uri)
    }

    pub(crate) fn iter(&self) -> impl ExactSizeIterator<Item = (&Uri, &Document)> {
        self.0.iter()
    }

    pub(crate) fn close(&mut self, uri: &Uri) -> Option<Document> {
        self.0.remove(uri)
    }
}

#[cfg(test)]
mod tests {
    use super::Documents;
    use lsp_types::{DidChangeTextDocumentParams, TextDocumentItem, Uri};
    use serde_json::json;

    fn opened() -> (Documents, Uri) {
        let mut documents = Documents::default();
        let uri: Uri = "untitled:document".parse().unwrap();
        documents.open(TextDocumentItem::new(
            uri.clone(),
            "iris".into(),
            4,
            "current".into(),
        ));
        (documents, uri)
    }

    #[test]
    fn preserves_text_when_versions_are_stale_or_equal() {
        let (mut documents, uri) = opened();
        for version in [3, 4] {
            let change =
                serde_json::from_value(json!({"textDocument":{"uri":uri,"version":version},
                "contentChanges":[{"text":"stale"}]}))
                .unwrap();
            assert!(documents.change(change).unwrap().is_none());
            assert_eq!(documents.get(&uri).unwrap().text, "current");
        }
    }

    #[test]
    fn uses_last_full_replacement_when_changes_are_ordered() {
        let (mut documents, uri) = opened();
        let change = serde_json::from_value(json!({"textDocument":{"uri":uri,"version":5},
            "contentChanges":[{"text":"first"},{"text":"last"}]}))
        .unwrap();
        documents.change(change).unwrap();
        let document = documents.get(&uri).unwrap();
        assert_eq!((&*document.text, document.version), ("last", 5));
    }

    #[test]
    fn preserves_document_when_incremental_change_is_rejected() {
        let (mut documents, uri) = opened();
        let change: DidChangeTextDocumentParams = serde_json::from_value(json!({
            "textDocument":{"uri":uri,"version":5},"contentChanges":[{"text":"new"},
            {"text":"bad","range":{"start":{"line":0,"character":0},"end":{"line":0,"character":1}}}]})).unwrap();
        assert!(documents.change(change).is_err());
        assert_eq!(documents.get(&uri).unwrap().text, "current");
    }

    #[test]
    fn forgets_document_when_closed() {
        let (mut documents, uri) = opened();
        let removed = documents.close(&uri);
        assert_eq!(removed.unwrap().text, "current");
    }

    #[test]
    fn replaces_generation_when_same_uri_and_version_are_reopened() {
        let (mut documents, uri) = opened();
        let generation = std::sync::Arc::clone(&documents.get(&uri).unwrap().generation);
        documents.close(&uri);

        documents.open(TextDocumentItem::new(
            uri.clone(),
            "iris".into(),
            4,
            "current".into(),
        ));

        assert!(!std::sync::Arc::ptr_eq(
            &generation,
            &documents.get(&uri).unwrap().generation
        ));
    }
}
