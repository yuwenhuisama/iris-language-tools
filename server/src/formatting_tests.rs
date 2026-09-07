use std::thread::JoinHandle;
use std::time::Duration;

use lsp_server::{Connection, Message};
use serde_json::{Value, json};

use crate::{
    session,
    worker::{Budget, Program},
};

struct Client {
    connection: Connection,
    server: Option<JoinHandle<anyhow::Result<std::process::ExitCode>>>,
}

impl Client {
    fn start(script: &str) -> Self {
        let (connection, server) = Connection::memory();
        let program = Program {
            executable: "/bin/sh".into(),
            arguments: vec!["-c".into(), script.into()],
        };
        let server = std::thread::spawn(move || {
            session::run_with_worker(&server, program, Budget::default())
        });
        let client = Self {
            connection,
            server: Some(server),
        };
        client.send(json!({"id":1,"method":"initialize","params":{"capabilities":{}}}));
        client.response();
        client.send(json!({"method":"initialized","params":{}}));
        client.open();
        client
    }

    fn send(&self, value: Value) {
        self.connection
            .sender
            .send(serde_json::from_value(value).unwrap())
            .unwrap();
    }

    fn response(&self) -> Value {
        loop {
            let message = self
                .connection
                .receiver
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            match message {
                Message::Response(response) => return serde_json::to_value(response).unwrap(),
                Message::Notification(_) => {}
                Message::Request(_) => panic!("unexpected request"),
            }
        }
    }

    fn open(&self) {
        self.send(
            json!({"method":"textDocument/didOpen","params":{"textDocument":{
            "uri":"untitled:worker", "languageId":"iris","version":1,"text":"let x=1"}}}),
        );
    }

    fn format(&self, id: i32) {
        self.send(json!({"id":id,"method":"textDocument/formatting","params":{
            "textDocument":{"uri":"untitled:worker"},"options":{"tabSize":8,"insertSpaces":false}}}));
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.send(json!({"method":"exit"}));
        if let Some(server) = self.server.take() {
            server.join().unwrap().unwrap();
        }
    }
}

#[test]
fn completes_requests_when_format_worker_is_stalled() {
    let client = Client::start("exec sleep 60");
    client.format(2);
    client.send(json!({"id":3,"method":"textDocument/completion","params":{
        "textDocument":{"uri":"untitled:worker"},"position":{"line":0,"character":0}}}));
    let response = client.response();
    assert_eq!(response["id"], 3);
    assert_eq!(response["result"].as_array().unwrap().len(), 50);
}

#[test]
fn rejects_additional_work_when_one_job_is_active() {
    let client = Client::start("exec sleep 60");
    client.format(2);
    client.format(3);
    let response = client.response();
    assert_eq!(response, json!({"id":3,"result":[]}));
}

#[test]
fn cancels_job_when_matching_request_id_is_notified() {
    let client = Client::start("exec sleep 60");
    client.format(2);
    client.send(json!({"method":"$/cancelRequest","params":{"id":2}}));
    let response = client.response();
    assert_eq!(response["id"], 2);
    assert_eq!(response["error"]["code"], -32800);
}

#[test]
fn discards_job_when_document_version_changes() {
    let client = Client::start("exec sleep 60");
    client.format(2);
    client.send(json!({"method":"textDocument/didChange","params":{
        "textDocument":{"uri":"untitled:worker","version":2},"contentChanges":[{"text":"let x=2"}]}}));
    let response = client.response();
    assert_eq!(response, json!({"id":2,"result":[]}));
}

#[test]
fn discards_job_when_document_is_closed_then_reopened_at_same_version() {
    let client = Client::start("exec sleep 60");
    client.format(2);
    client.send(json!({"method":"textDocument/didClose","params":{"textDocument":{"uri":"untitled:worker"}}}));
    client.open();
    let response = client.response();
    assert_eq!(response, json!({"id":2,"result":[]}));
}

#[test]
fn cleans_active_worker_when_shutdown_is_requested() {
    let client = Client::start("exec sleep 60");
    client.format(2);
    client.send(json!({"id":3,"method":"shutdown","params":null}));
    let cancelled = client.response();
    let shutdown = client.response();
    assert_eq!(cancelled["error"]["code"], -32800);
    assert_eq!(shutdown, json!({"id":3,"result":null}));
}

#[test]
fn returns_no_edit_when_worker_crashes_and_accepts_later_requests() {
    let client = Client::start("exit 17");
    client.format(2);

    let response = client.response();

    assert_eq!(response, json!({"id":2,"result":[]}));
    client.send(json!({"id":3,"method":"shutdown","params":null}));
    assert_eq!(client.response(), json!({"id":3,"result":null}));
}
