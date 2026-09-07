use super::*;
use crate::{hover::Format, semantic_query::Operation, semantic_worker::Finished};
use lsp_server::Message;
use serde_json::json;

fn coordinator(worker: SemanticWorker) -> Semantics {
    Semantics {
        worker,
        pending: Vec::new(),
        epoch: Epoch(0),
        next_ticket: 0,
        roots: Vec::new(),
        hover_format: Format::Markdown,
        inputs: None,
    }
}

fn request() -> Request {
    Request::new(
        1.into(),
        "textDocument/hover".into(),
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
fn cancels_hover_once_when_worker_returns_late() {
    let (worker, jobs, finished) = SemanticWorker::controlled();
    let mut given = coordinator(worker);
    let documents = Documents::default();
    let (server, client) = Connection::memory();
    given.request(request(), (&server, &documents)).unwrap();
    let job = jobs.recv().unwrap();
    assert!(matches!(
        job.query.operation,
        Operation::Hover(_, Format::Markdown)
    ));
    given.cancel(&server, json!({"id":1})).unwrap();
    finished
        .send(Finished {
            ticket: job.ticket,
            result: Ok(json!({"contents":"late"})),
        })
        .unwrap();

    given.poll(&server, &documents).unwrap();

    assert_eq!(response(&client)["error"]["code"], -32800);
    assert!(client.receiver.try_recv().is_err());
    assert!(job.cancelled.load(Ordering::Relaxed));
}

#[test]
fn rejects_hover_when_workspace_epoch_changes() {
    let (worker, jobs, finished) = SemanticWorker::controlled();
    let mut given = coordinator(worker);
    let documents = Documents::default();
    let (server, client) = Connection::memory();
    given.request(request(), (&server, &documents)).unwrap();
    let job = jobs.recv().unwrap();
    given.changed().unwrap();
    finished
        .send(Finished {
            ticket: job.ticket,
            result: Ok(json!({"contents":"stale"})),
        })
        .unwrap();

    given.poll(&server, &documents).unwrap();

    assert_eq!(response(&client)["error"]["code"], -32801);
    assert!(client.receiver.try_recv().is_err());
}

#[test]
fn retains_cached_inputs_when_only_hover_format_changes() {
    let (worker, jobs, _finished) = SemanticWorker::controlled();
    let mut given = coordinator(worker);
    let documents = Documents::default();
    let (server, _client) = Connection::memory();
    given.request(request(), (&server, &documents)).unwrap();
    let first = jobs.recv().unwrap();
    given.hover_format = Format::Plaintext;

    given.request(request(), (&server, &documents)).unwrap();

    let second = jobs.recv().unwrap();
    assert!(Arc::ptr_eq(&first.inputs, &second.inputs));
    assert!(matches!(
        second.query.operation,
        Operation::Hover(_, Format::Plaintext)
    ));
}
