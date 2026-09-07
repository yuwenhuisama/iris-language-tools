use super::*;
use crate::semantic_worker::Finished;
use lsp_server::Message;
use serde_json::json;

fn coordinator(worker: SemanticWorker) -> Semantics {
    Semantics {
        worker,
        pending: Vec::new(),
        epoch: Epoch(0),
        next_ticket: 0,
        roots: Vec::new(),
        hover_format: crate::hover::Format::default(),
        inputs: None,
    }
}

fn request(id: i32) -> Request {
    Request::new(
        id.into(),
        "textDocument/definition".into(),
        json!({
        "textDocument":{"uri":"untitled:test"},"position":{"line":0,"character":0}}),
    )
}

fn response(connection: &Connection) -> serde_json::Value {
    match connection.receiver.try_recv().unwrap() {
        Message::Response(response) => serde_json::to_value(response).unwrap(),
        message => panic!("unexpected {message:?}"),
    }
}

#[test]
fn responds_once_when_cancelled_job_finishes_later() {
    let (worker, jobs, finished) = SemanticWorker::controlled();
    let mut given = coordinator(worker);
    let documents = Documents::default();
    let (server, client) = Connection::memory();
    given.request(request(1), (&server, &documents)).unwrap();
    let job = jobs.recv().unwrap();
    given.cancel(&server, json!({"id":1})).unwrap();
    finished
        .send(Finished {
            ticket: job.ticket,
            result: Ok(json!([])),
        })
        .unwrap();
    given.poll(&server, &documents).unwrap();
    assert_eq!(response(&client)["error"]["code"], -32800);
    assert!(client.receiver.try_recv().is_err());
    assert!(job.cancelled.load(Ordering::Relaxed));
}

#[test]
fn rejects_stale_result_when_workspace_epoch_changes() {
    let (worker, jobs, finished) = SemanticWorker::controlled();
    let mut given = coordinator(worker);
    let documents = Documents::default();
    let (server, client) = Connection::memory();
    given.request(request(1), (&server, &documents)).unwrap();
    let job = jobs.recv().unwrap();
    given.changed().unwrap();
    finished
        .send(Finished {
            ticket: job.ticket,
            result: Ok(json!([])),
        })
        .unwrap();
    given.poll(&server, &documents).unwrap();
    assert_eq!(response(&client)["error"]["code"], -32801);
    assert!(client.receiver.try_recv().is_err());
}

#[test]
fn bounds_outstanding_work_when_cancelled_jobs_await_acknowledgement() {
    let (worker, _jobs, _finished) = SemanticWorker::controlled();
    let mut given = coordinator(worker);
    let documents = Documents::default();
    let (server, client) = Connection::memory();
    for id in 0..32 {
        given.request(request(id), (&server, &documents)).unwrap();
        given.cancel(&server, json!({"id":id})).unwrap();
        assert_eq!(response(&client)["error"]["code"], -32800);
    }
    given.request(request(32), (&server, &documents)).unwrap();
    assert_eq!(response(&client)["error"]["code"], -32802);
    assert_eq!(given.pending.len(), CAPACITY);
}

#[test]
fn cancels_all_pending_when_shutdown_is_accepted() {
    let (worker, jobs, _finished) = SemanticWorker::controlled();
    let mut given = coordinator(worker);
    let documents = Documents::default();
    let (server, client) = Connection::memory();
    given.request(request(1), (&server, &documents)).unwrap();
    let job = jobs.recv().unwrap();
    given.stop(&server).unwrap();
    assert_eq!(response(&client)["error"]["code"], -32800);
    assert!(job.cancelled.load(Ordering::Relaxed));
}

#[test]
fn rejects_generation_when_document_reopens_at_same_version_without_epoch_change() {
    let (worker, jobs, finished) = SemanticWorker::controlled();
    let mut given = coordinator(worker);
    let mut documents = Documents::default();
    let uri: Uri = "untitled:test".parse().unwrap();
    let item = lsp_types::TextDocumentItem::new(uri.clone(), "iris".into(), 1, String::new());
    documents.open(item.clone());
    let (server, client) = Connection::memory();
    given.request(request(1), (&server, &documents)).unwrap();
    let job = jobs.recv().unwrap();
    documents.close(&uri);
    documents.open(item);
    finished
        .send(Finished {
            ticket: job.ticket,
            result: Ok(json!([])),
        })
        .unwrap();
    given.poll(&server, &documents).unwrap();
    assert_eq!(response(&client)["error"]["code"], -32801);
}

#[test]
fn resolves_package_when_real_workspace_is_loaded_by_worker() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    std::fs::write(root.join("iris.toml"), "manifest_version = 1\npackage_id = \"org.example.semantic\"\napi_major = 1\nversion = \"1.0.0\"\niris_major = 1\nsources = [\"main.iris\", \"types.iris\"]\nentry_modules = []\n[permissions]\nrequired = []\noptional = []\n").unwrap();
    std::fs::write(
        root.join("main.iris"),
        "module Main { let item = Box.new() }",
    )
    .unwrap();
    std::fs::write(root.join("types.iris"), "class Box {}").unwrap();
    let root_uri: Uri = url::Url::from_file_path(&root)
        .unwrap()
        .as_str()
        .parse()
        .unwrap();
    let source_uri: Uri = url::Url::from_file_path(root.join("main.iris"))
        .unwrap()
        .as_str()
        .parse()
        .unwrap();
    let mut workspace = crate::workspace::Workspace::new(vec![root_uri]);
    workspace.reload(&OverlaySet::default()).unwrap();
    let snapshot = workspace.snapshot();
    assert!(snapshot.is_complete(), "{:?}", snapshot.issues);
    assert_eq!(snapshot.files.len(), 2);
    assert_eq!(snapshot.files[0].group, snapshot.files[1].group);
    let analysis = iris_analysis::AnalysisSnapshot::new(snapshot.files.iter().enumerate().map(
        |(index, file)| iris_analysis::SourceInput {
            id: iris_analysis::FileId(u32::try_from(index).unwrap()),
            group: iris_analysis::GroupId(0),
            text: Arc::clone(&file.text),
        },
    ));
    let query = Query {
        uri: source_uri,
        operation: crate::semantic_query::Operation::Definition(lsp_types::Position::new(0, 25)),
    };
    let when = query.execute((&snapshot, &analysis), &[]).unwrap();
    assert_eq!(when.as_array().unwrap().len(), 1, "{when:?}");
}
