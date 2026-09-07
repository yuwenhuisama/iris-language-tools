use super::*;
use crate::documents::Documents;
use lsp_types::TextDocumentItem;
use std::{fs, path::Path};

pub(super) fn uri(path: &Path) -> Uri {
    url::Url::from_file_path(path)
        .unwrap()
        .as_str()
        .parse()
        .unwrap()
}

pub(super) fn manifest(root: &Path, sources: &[&str]) {
    fs::create_dir_all(root).unwrap();
    fs::write(root.join("iris.toml"), format!(
        "manifest_version = 1\npackage_id = \"org.example.test\"\napi_major = 1\nversion = \"1.0.0\"\niris_major = 1\nsources = {sources:?}\nentry_modules = []\n[permissions]\nrequired = []\noptional = []\n"
    )).unwrap();
}

#[test]
fn shares_manifest_identity_when_package_contains_two_sources() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    manifest(&root, &["first %.iris", "second.iris"]);
    fs::write(root.join("first %.iris"), "first disk").unwrap();
    fs::write(root.join("second.iris"), "second disk").unwrap();
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace
        .reload(&OverlaySet::capture(&Documents::default()))
        .unwrap();

    let snapshot = workspace.snapshot();
    assert!(snapshot.is_complete(), "{:?}", snapshot.issues);
    assert_eq!(snapshot.files.len(), 2);
    assert_eq!(snapshot.files[0].group, snapshot.files[1].group);
    assert!(
        matches!(&snapshot.files[0].group, GroupIdentity::Package(identity)
        if identity.package_id.as_str() == "org.example.test" && identity.api_major == 1)
    );
    assert_eq!(
        &*snapshot
            .file(&uri(&root.join("first %.iris")))
            .unwrap()
            .text,
        "first disk"
    );
}

#[test]
fn preserves_exact_buffer_when_disk_source_is_deleted() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    manifest(&root, &["first.iris"]);
    let source = uri(&root.join("first.iris"));
    let mut documents = Documents::default();
    documents.open(TextDocumentItem::new(
        source.clone(),
        "iris".into(),
        7,
        "\u{feff}dirty\r\n\u{1f600}".into(),
    ));
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace.reload(&OverlaySet::capture(&documents)).unwrap();

    let snapshot = workspace.snapshot();
    assert!(snapshot.is_complete(), "{:?}", snapshot.issues);
    assert_eq!(
        &*snapshot.file(&source).unwrap().text,
        "\u{feff}dirty\r\n\u{1f600}"
    );
    assert!(matches!(
        snapshot.file(&source).unwrap().origin,
        SourceOrigin::Buffer {
            version: DocumentVersion(7),
            ..
        }
    ));
}
