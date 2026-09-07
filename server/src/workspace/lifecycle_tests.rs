use super::{
    tests::{manifest, uri},
    *,
};
use crate::documents::Documents;
use lsp_types::TextDocumentItem;
use serde_json::json;
use std::fs;

#[test]
fn restores_disk_when_open_document_closes() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    manifest(&root, &["source.iris"]);
    fs::write(root.join("source.iris"), "disk").unwrap();
    let source = uri(&root.join("source.iris"));
    let mut documents = Documents::default();
    documents.open(TextDocumentItem::new(
        source.clone(),
        "iris".into(),
        4,
        "dirty".into(),
    ));
    let mut workspace = Workspace::new(vec![uri(&root)]);
    workspace.reload(&OverlaySet::capture(&documents)).unwrap();
    let before = workspace.snapshot();
    documents.close(&source);

    workspace.reload(&OverlaySet::capture(&documents)).unwrap();

    let after = workspace.snapshot();
    assert_eq!(&*after.file(&source).unwrap().text, "disk");
    assert_eq!(&*before.file(&source).unwrap().text, "dirty");
    assert!(after.revision > before.revision);
}

#[test]
fn keeps_revision_when_equal_or_stale_changes_are_rejected() {
    let source: Uri = "untitled:source".parse().unwrap();
    let mut documents = Documents::default();
    documents.open(TextDocumentItem::new(
        source.clone(),
        "iris".into(),
        4,
        "current".into(),
    ));
    let mut workspace = Workspace::new(Vec::new());
    let before = workspace.reload(&OverlaySet::capture(&documents)).unwrap();

    for version in [3, 4] {
        documents
            .change(
                serde_json::from_value(json!({"textDocument": {"uri": source, "version": version},
            "contentChanges": [{"text": "stale"}]}))
                .unwrap(),
            )
            .unwrap();
        let after = workspace.reload(&OverlaySet::capture(&documents)).unwrap();

        assert_eq!(before, after);
        assert_eq!(
            &*workspace.snapshot().file(&source).unwrap().text,
            "current"
        );
    }
}

#[test]
fn advances_revision_when_newer_version_changes_source() {
    let source: Uri = "untitled:source".parse().unwrap();
    let mut documents = Documents::default();
    documents.open(TextDocumentItem::new(
        source.clone(),
        "iris".into(),
        4,
        "current".into(),
    ));
    let mut workspace = Workspace::new(Vec::new());
    let before = workspace.reload(&OverlaySet::capture(&documents)).unwrap();
    documents
        .change(
            serde_json::from_value(json!({"textDocument": {"uri": source, "version": 5},
        "contentChanges": [{"text": "new"}]}))
            .unwrap(),
        )
        .unwrap();

    let after = workspace.reload(&OverlaySet::capture(&documents)).unwrap();

    assert!(after > before);
    assert_eq!(&*workspace.snapshot().file(&source).unwrap().text, "new");
}

#[test]
fn changes_generation_when_same_version_and_text_are_reopened_between_reloads() {
    let source: Uri = "untitled:source".parse().unwrap();
    let item = TextDocumentItem::new(source.clone(), "iris".into(), 4, "same".into());
    let mut documents = Documents::default();
    documents.open(item.clone());
    let mut workspace = Workspace::new(Vec::new());
    workspace.reload(&OverlaySet::capture(&documents)).unwrap();
    let before = workspace.snapshot();
    documents.close(&source);
    documents.open(item);

    workspace.reload(&OverlaySet::capture(&documents)).unwrap();

    let after = workspace.snapshot();
    assert!(after.revision > before.revision);
    assert_ne!(after.files[0].origin, before.files[0].origin);
}

#[test]
fn isolates_open_documents_when_root_is_removed() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    manifest(&root, &["first.iris", "second.iris"]);
    fs::write(root.join("second.iris"), "second").unwrap();
    let first = uri(&root.join("first.iris"));
    let untitled: Uri = "untitled:outside".parse().unwrap();
    let mut documents = Documents::default();
    for source in [&first, &untitled] {
        documents.open(TextDocumentItem::new(
            source.clone(),
            "iris".into(),
            1,
            "open".into(),
        ));
    }
    let mut workspace = Workspace::new(vec![uri(&root)]);
    workspace.reload(&OverlaySet::capture(&documents)).unwrap();

    workspace
        .replace_roots(Vec::new(), &OverlaySet::capture(&documents))
        .unwrap();

    let snapshot = workspace.snapshot();
    assert!(snapshot.is_complete());
    assert_eq!(snapshot.files.len(), 2);
    assert!(
        matches!(&snapshot.file(&first).unwrap().group, GroupIdentity::Standalone(identity) if identity == &first)
    );
    assert_ne!(snapshot.files[0].group, snapshot.files[1].group);
}

#[test]
fn replaces_membership_when_manifest_is_reloaded() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    manifest(&root, &["first.iris"]);
    fs::write(root.join("first.iris"), "first").unwrap();
    fs::write(root.join("second.iris"), "second").unwrap();
    let mut workspace = Workspace::new(vec![uri(&root)]);
    let overlays = OverlaySet::default();
    let before = workspace.reload(&overlays).unwrap();
    manifest(&root, &["second.iris"]);

    let after = workspace.reload(&overlays).unwrap();

    assert!(after > before);
    assert_eq!(workspace.snapshot().files.len(), 1);
    assert_eq!(&*workspace.snapshot().files[0].text, "second");
}

#[test]
fn reports_missing_source_when_deleted_buffer_closes() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    manifest(&root, &["missing.iris"]);
    let source = uri(&root.join("missing.iris"));
    let mut documents = Documents::default();
    documents.open(TextDocumentItem::new(
        source.clone(),
        "iris".into(),
        1,
        "dirty".into(),
    ));
    let mut workspace = Workspace::new(vec![uri(&root)]);
    workspace.reload(&OverlaySet::capture(&documents)).unwrap();
    documents.close(&source);

    workspace.reload(&OverlaySet::capture(&documents)).unwrap();

    let snapshot = workspace.snapshot();
    assert!(snapshot.files.is_empty());
    assert!(
        snapshot
            .issues
            .iter()
            .any(|issue| issue.kind == IssueKind::Io(std::io::ErrorKind::NotFound))
    );
}
