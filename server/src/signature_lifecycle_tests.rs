use super::*;
use crate::{semantic_query::Operation, semantic_worker::Finished, signature_help::Options};
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
        signature_options: Options {
            format: crate::hover::Format::Markdown,
            label_offsets: true,
        },
        inputs: None,
    }
}

fn request(id: i32) -> Request {
    Request::new(
        id.into(),
        "textDocument/signatureHelp".into(),
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
fn cancels_signature_once_when_worker_returns_late() {
    let (worker, jobs, finished) = SemanticWorker::controlled();
    let mut given = coordinator(worker);
    let documents = Documents::default();
    let (server, client) = Connection::memory();
    given.request(request(1), (&server, &documents)).unwrap();
    let job = jobs.recv().unwrap();
    assert!(
        matches!(job.query.operation, Operation::SignatureHelp(_, options)
        if options == given.signature_options)
    );
    given.cancel(&server, json!({"id":1})).unwrap();
    finished
        .send(Finished {
            ticket: job.ticket,
            result: Ok(json!({"signatures":[]})),
        })
        .unwrap();

    given.poll(&server, &documents).unwrap();

    assert_eq!(response(&client)["error"]["code"], -32800);
    assert!(client.receiver.try_recv().is_err());
    assert!(job.cancelled.load(Ordering::Relaxed));
}

#[test]
fn rejects_signature_when_workspace_epoch_changes() {
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
            result: Ok(json!({"signatures":[]})),
        })
        .unwrap();

    given.poll(&server, &documents).unwrap();

    assert_eq!(response(&client)["error"]["code"], -32801);
    assert!(client.receiver.try_recv().is_err());
}

#[test]
fn rejects_signature_when_document_reopens_at_same_version() {
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
            result: Ok(json!({"signatures":[]})),
        })
        .unwrap();

    given.poll(&server, &documents).unwrap();

    assert_eq!(response(&client)["error"]["code"], -32801);
    assert!(client.receiver.try_recv().is_err());
}

#[test]
fn bounds_signature_admission_when_cancelled_jobs_are_still_pending() {
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
