use super::{
    tests::{manifest, uri},
    *,
};
use std::fs;

#[cfg(unix)]
#[test]
fn skips_symlink_directories_when_discovering_external_packages() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let outside = tempfile::tempdir().unwrap();
    manifest(outside.path(), &["secret.iris"]);
    fs::write(outside.path().join("secret.iris"), "secret").unwrap();
    std::os::unix::fs::symlink(outside.path(), root.join("linked")).unwrap();
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace.reload(&OverlaySet::default()).unwrap();

    assert!(workspace.snapshot().files.is_empty());
    assert!(workspace.snapshot().is_complete());
}

#[test]
fn retains_uri_and_overlay_when_filename_contains_percent_spaces_and_unicode() {
    use crate::documents::Documents;
    use lsp_types::TextDocumentItem;
    let directory = tempfile::tempdir().unwrap();
    let root = directory
        .path()
        .canonicalize()
        .unwrap()
        .join("space % root");
    let name = "space % \u{1f600}.iris";
    manifest(&root, &[name]);
    fs::write(root.join(name), "disk").unwrap();
    let source = uri(&root.join(name));
    let mut documents = Documents::default();
    documents.open(TextDocumentItem::new(
        source.clone(),
        "iris".into(),
        1,
        "buffer".into(),
    ));
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace.reload(&OverlaySet::capture(&documents)).unwrap();

    let snapshot = workspace.snapshot();
    assert!(snapshot.is_complete());
    assert_eq!(snapshot.files.len(), 1);
    assert_eq!(&*snapshot.file(&source).unwrap().text, "buffer");
    assert!(matches!(snapshot.files[0].group, GroupIdentity::Package(_)));
}

#[test]
fn marks_all_claimants_ambiguous_when_package_identity_is_duplicated() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    for name in ["one", "two"] {
        let package = root.join(name);
        manifest(&package, &["source.iris"]);
        fs::write(package.join("source.iris"), name).unwrap();
    }
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace.reload(&OverlaySet::default()).unwrap();

    let snapshot = workspace.snapshot();
    assert!(!snapshot.is_complete());
    assert_eq!(snapshot.files.len(), 2);
    assert!(
        snapshot
            .files
            .iter()
            .all(|file| matches!(file.group, GroupIdentity::Ambiguous(_)))
    );
}

#[test]
fn marks_shared_source_ambiguous_when_nested_manifests_claim_it() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    manifest(&root, &["nested/source.iris"]);
    manifest(&root.join("nested"), &["source.iris"]);
    let nested = root.join("nested/iris.toml");
    fs::write(
        &nested,
        fs::read_to_string(&nested)
            .unwrap()
            .replace("org.example.test", "org.example.other"),
    )
    .unwrap();
    fs::write(root.join("nested/source.iris"), "shared").unwrap();
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace.reload(&OverlaySet::default()).unwrap();

    let snapshot = workspace.snapshot();
    assert_eq!(snapshot.files.len(), 1);
    assert!(matches!(
        snapshot.files[0].group,
        GroupIdentity::Ambiguous(_)
    ));
    assert!(
        snapshot
            .issues
            .iter()
            .any(|issue| issue.kind == IssueKind::DuplicateSource)
    );
}

#[test]
fn ignores_excluded_directories_when_discovering_manifests() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    for name in [".git", ".iris", "target", "node_modules"] {
        manifest(&root.join(name), &["missing.iris"]);
    }
    let mut workspace = Workspace::new(vec![uri(&root), uri(&root)]);

    workspace.reload(&OverlaySet::default()).unwrap();

    assert!(workspace.snapshot().is_complete());
    assert!(workspace.snapshot().files.is_empty());
}

#[test]
fn reports_incomplete_when_manifest_or_source_is_invalid() {
    for case in ["manifest", "utf8", "directory", "escape"] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        manifest(&root, &["source.iris"]);
        match case {
            "manifest" => fs::write(root.join("iris.toml"), "invalid").unwrap(),
            "utf8" => fs::write(root.join("source.iris"), [255]).unwrap(),
            "directory" => fs::create_dir(root.join("source.iris")).unwrap(),
            "escape" => manifest(&root, &["../source.iris"]),
            _ => unreachable!(),
        }
        let mut workspace = Workspace::new(vec![uri(&root)]);

        workspace.reload(&OverlaySet::default()).unwrap();

        assert!(!workspace.snapshot().is_complete(), "{case}");
        assert!(workspace.snapshot().files.is_empty());
    }
}

#[cfg(unix)]
#[test]
fn rejects_source_symlinks_when_they_escape_or_stay_within_root() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret.iris"), "outside").unwrap();
    fs::write(root.join("real.iris"), "inside").unwrap();
    symlink(outside.path(), root.join("linked-dir")).unwrap();
    symlink(root.join("real.iris"), root.join("linked.iris")).unwrap();
    manifest(&root, &["linked-dir/secret.iris", "linked.iris"]);
    let mut workspace = Workspace::new(vec![uri(&root)]);

    workspace.reload(&OverlaySet::default()).unwrap();

    let snapshot = workspace.snapshot();
    assert!(snapshot.files.is_empty());
    assert_eq!(
        snapshot
            .issues
            .iter()
            .filter(|issue| issue.kind == IssueKind::UnsafePath)
            .count(),
        2
    );
}

#[test]
fn rejects_non_file_roots_when_uri_cannot_name_local_directory() {
    let mut workspace = Workspace::new(vec!["https://example.com/workspace".parse().unwrap()]);

    workspace.reload(&OverlaySet::default()).unwrap();

    assert_eq!(workspace.snapshot().issues[0].kind, IssueKind::InvalidUri);
}
