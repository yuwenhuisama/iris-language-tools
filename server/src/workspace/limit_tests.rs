use super::{
    tests::{manifest, uri},
    *,
};
use crate::documents::Documents;
use lsp_types::TextDocumentItem;
use std::fs;

#[test]
fn reports_truncation_when_disk_total_bytes_exceed_limit() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let sources: Vec<_> = (0..=MAX_TOTAL_BYTES / MAX_FILE_BYTES)
        .map(|index| format!("{index:04}.iris"))
        .collect();
    for source in &sources {
        fs::write(root.join(source), vec![b'a'; MAX_FILE_BYTES]).unwrap();
    }
    manifest(
        &root,
        &sources.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace.reload(&OverlaySet::default()).unwrap();

    let snapshot = workspace.snapshot();
    assert!(
        snapshot
            .files
            .iter()
            .map(|file| file.text.len())
            .sum::<usize>()
            <= MAX_TOTAL_BYTES
    );
    assert!(
        snapshot
            .issues
            .iter()
            .any(|issue| issue.kind == IssueKind::TotalBytes)
    );
}

#[test]
fn admits_disk_file_when_size_equals_per_file_limit() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    manifest(&root, &["exact.iris"]);
    fs::write(root.join("exact.iris"), vec![b'a'; MAX_FILE_BYTES]).unwrap();
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace.reload(&OverlaySet::default()).unwrap();

    let snapshot = workspace.snapshot();
    assert!(snapshot.is_complete());
    assert_eq!(snapshot.files[0].text.len(), MAX_FILE_BYTES);
}

#[test]
fn excludes_disk_source_when_per_file_limit_is_exceeded() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    manifest(&root, &["large.iris"]);
    fs::write(root.join("large.iris"), vec![b'a'; MAX_FILE_BYTES + 1]).unwrap();
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace.reload(&OverlaySet::default()).unwrap();

    assert!(workspace.snapshot().files.is_empty());
    assert_eq!(workspace.snapshot().issues[0].kind, IssueKind::FileBytes);
}

#[test]
fn excludes_disk_fallback_when_buffer_exceeds_per_file_limit() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    manifest(&root, &["source.iris"]);
    fs::write(root.join("source.iris"), "stale disk").unwrap();
    let mut documents = Documents::default();
    documents.open(TextDocumentItem::new(
        uri(&root.join("source.iris")),
        "iris".into(),
        1,
        "a".repeat(MAX_FILE_BYTES + 1),
    ));
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace.reload(&OverlaySet::capture(&documents)).unwrap();

    assert!(workspace.snapshot().files.is_empty());
    assert!(
        workspace
            .snapshot()
            .issues
            .iter()
            .any(|issue| issue.kind == IssueKind::FileBytes)
    );
}

#[test]
fn reports_truncation_when_overlay_file_count_exceeds_limit() {
    let mut documents = Documents::default();
    for index in 0..=MAX_FILES {
        documents.open(TextDocumentItem::new(
            format!("untitled:{index:04}").parse().unwrap(),
            "iris".into(),
            1,
            "a".into(),
        ));
    }
    let mut workspace = Workspace::new(Vec::new());

    workspace.reload(&OverlaySet::capture(&documents)).unwrap();

    assert_eq!(workspace.snapshot().files.len(), MAX_FILES);
    assert_eq!(workspace.snapshot().issues[0].kind, IssueKind::FileCount);
}

#[test]
fn reports_truncation_when_overlay_total_bytes_exceed_limit() {
    let mut documents = Documents::default();
    for index in 0..=MAX_TOTAL_BYTES / MAX_FILE_BYTES {
        documents.open(TextDocumentItem::new(
            format!("untitled:{index:04}").parse().unwrap(),
            "iris".into(),
            1,
            "a".repeat(MAX_FILE_BYTES),
        ));
    }
    let mut workspace = Workspace::new(Vec::new());

    workspace.reload(&OverlaySet::capture(&documents)).unwrap();

    let snapshot = workspace.snapshot();
    assert_eq!(
        snapshot
            .files
            .iter()
            .map(|file| file.text.len())
            .sum::<usize>(),
        MAX_TOTAL_BYTES
    );
    assert_eq!(snapshot.issues[0].kind, IssueKind::TotalBytes);
}

#[test]
fn reports_truncation_when_manifest_source_count_exceeds_limit() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let sources: Vec<_> = (0..=MAX_FILES)
        .map(|index| format!("{index:04}.iris"))
        .collect();
    for source in &sources {
        fs::write(root.join(source), "a").unwrap();
    }
    manifest(
        &root,
        &sources.iter().map(String::as_str).collect::<Vec<_>>(),
    );
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace.reload(&OverlaySet::default()).unwrap();

    assert_eq!(workspace.snapshot().files.len(), MAX_FILES);
    assert!(
        workspace
            .snapshot()
            .issues
            .iter()
            .any(|issue| issue.kind == IssueKind::FileCount)
    );
}

#[test]
fn reports_truncation_when_discovery_entry_budget_is_exhausted() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    for index in 0..=MAX_DISCOVERY_ENTRIES {
        fs::write(root.join(index.to_string()), "").unwrap();
    }
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace.reload(&OverlaySet::default()).unwrap();

    assert_eq!(
        workspace.snapshot().issues[0].kind,
        IssueKind::DiscoveryEntries
    );
}
