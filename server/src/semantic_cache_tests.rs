use super::*;
use crate::semantic_query::Operation;
use lsp_types::Position;
use serde_json::json;

#[test]
fn reuses_index_when_disk_changes_without_an_accepted_refresh() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    std::fs::write(root.join("iris.toml"), "manifest_version = 1\npackage_id = \"org.example.cache\"\napi_major = 1\nversion = \"1.0.0\"\niris_major = 1\nsources = [\"main.iris\"]\nentry_modules = []\n[permissions]\nrequired = []\noptional = []\n").unwrap();
    let path = root.join("main.iris");
    std::fs::write(&path, "module Main { let local = 1; local }").unwrap();
    let uri: Uri = url::Url::from_file_path(&path)
        .unwrap()
        .as_str()
        .parse()
        .unwrap();
    let root_uri: Uri = url::Url::from_file_path(&root)
        .unwrap()
        .as_str()
        .parse()
        .unwrap();
    let inputs = Arc::new(Inputs {
        epoch: Epoch(1),
        roots: vec![root_uri],
        overlays: OverlaySet::default(),
    });
    let query = Query {
        uri,
        operation: Operation::Definition(Position::new(0, 30)),
    };
    let job = Job {
        ticket: Ticket(1),
        inputs,
        query,
        cancelled: Arc::new(AtomicBool::new(false)),
    };
    let mut workspace = Workspace::new(Vec::new());
    let mut roots = Vec::new();
    let mut cache = None;
    let initial = execute(&job, (&mut workspace, &mut roots, &mut cache), &[]).unwrap();
    assert_eq!(initial.as_array().unwrap().len(), 1);
    std::fs::write(path, "module Main { let other = 1; other }").unwrap();
    let when = execute(&job, (&mut workspace, &mut roots, &mut cache), &[]).unwrap();
    assert_eq!(when, initial);
    assert_eq!(
        cache.as_ref().unwrap().snapshot.revision,
        workspace.snapshot().revision
    );
}

#[test]
fn skips_inventory_when_job_was_cancelled_before_worker_admission() {
    let job = Job {
        ticket: Ticket(1),
        inputs: Arc::new(Inputs {
            epoch: Epoch(1),
            roots: vec!["file:///nonexistent-semantic-root".parse().unwrap()],
            overlays: OverlaySet::default(),
        }),
        query: Query {
            uri: "untitled:test".parse().unwrap(),
            operation: Operation::Definition(Position::new(0, 0)),
        },
        cancelled: Arc::new(AtomicBool::new(true)),
    };
    let mut workspace = Workspace::new(Vec::new());
    let mut roots = Vec::new();
    let mut cache = None;
    let when = execute(&job, (&mut workspace, &mut roots, &mut cache), &[]);
    assert!(matches!(when, Err(Failure::Cancelled)));
    assert!(cache.is_none());
    assert_eq!(json!(workspace.snapshot().revision.0), 0);
}
