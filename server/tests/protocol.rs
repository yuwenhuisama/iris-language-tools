mod support;

use serde_json::json;
use support::Client;

#[test]
fn advertises_full_sync_and_keywords_when_initialized() {
    let mut client = Client::spawn();
    let initialized = client.initialize();
    assert_eq!(
        initialized["result"]["capabilities"]["positionEncoding"],
        "utf-16"
    );
    assert_eq!(
        initialized["result"]["capabilities"]["textDocumentSync"]["change"],
        1
    );
    client.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion",
        "params":{"textDocument":{"uri":"untitled:one"},"position":{"line":0,"character":0}}}),
    );
    let response = client.response();
    let labels: Vec<_> = response["result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            assert_eq!(item["kind"], 14);
            item["label"].as_str().unwrap()
        })
        .collect();
    let expected: Vec<String> =
        serde_json::from_str(include_str!("../../language/keywords.json")).unwrap();
    assert_eq!(labels, expected);
    assert_eq!(labels.len(), 50);
    client.shutdown();
}

#[test]
fn clears_diagnostics_when_unicode_document_is_fixed_and_closed() {
    let mut client = Client::spawn();
    client.initialize();
    let uri = "untitled:unicode";
    let opened = client.open(uri, "\u{feff}// lead\r\nlet text = \"\u{1f600}\"; \\");
    assert_eq!(opened["params"]["version"], 1);
    assert_eq!(
        opened["params"]["diagnostics"][0]["code"],
        "LEX_BAD_CONTINUATION"
    );
    assert_eq!(
        opened["params"]["diagnostics"][0]["range"],
        json!({
        "start":{"line":1,"character":17},"end":{"line":1,"character":17}})
    );
    client.send(
        &json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
        "textDocument":{"uri":uri,"version":2},"contentChanges":[{"text":"let fixed = 1"}]}}),
    );
    let fixed = client.receive();
    assert_eq!(fixed["params"]["version"], 2);
    assert_eq!(fixed["params"]["diagnostics"], json!([]));
    client.send(
        &json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
        "textDocument":{"uri":uri,"version":1},"contentChanges":[{"text":"\\"}]}}),
    );
    client.send(&json!({"jsonrpc":"2.0","method":"textDocument/didClose",
        "params":{"textDocument":{"uri":uri}}}));
    assert_eq!(client.receive()["params"]["diagnostics"], json!([]));
    client.shutdown();
}

#[test]
fn logs_unlocated_literal_errors_when_scanner_has_placeholder_offsets() {
    let mut client = Client::spawn();
    client.initialize();
    let uri = "file:///tmp/literal.iris";
    let publication = client.open(uri, "let valid = 1\nlet bad = 1__0");
    assert_eq!(publication["params"]["diagnostics"], json!([]));
    let log = client.receive();
    assert_eq!(log["method"], "window/logMessage");
    let event: serde_json::Value =
        serde_json::from_str(log["params"]["message"].as_str().unwrap()).unwrap();
    assert_eq!(event["code"], "LEX_BAD_NUMERIC_SEPARATOR");
    assert_eq!(event["uri"], uri);
    assert_eq!(event["version"], 1);
    client.shutdown();
}

#[test]
fn responds_with_errors_when_requests_are_malformed_or_unknown() {
    let mut client = Client::spawn();
    client.initialize();
    for (method, code) in [
        ("textDocument/completion", -32602),
        ("iris/unknown", -32601),
    ] {
        client.send(&json!({"jsonrpc":"2.0","id":3,"method":method,"params":{}}));
        assert_eq!(client.receive()["error"]["code"], code);
    }
    client.send(&json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{}}));
    assert_eq!(client.receive()["method"], "window/logMessage");
    client.shutdown();
}

#[test]
fn exits_with_failure_when_exit_precedes_shutdown() {
    let mut client = Client::spawn();
    client.initialize();
    client.exit(1);
}

#[test]
fn recovers_when_initialize_parameters_are_malformed() {
    let mut client = Client::spawn();
    client.send(&json!({"jsonrpc":"2.0","id":4,"method":"textDocument/completion","params":{}}));
    assert_eq!(client.receive()["error"]["code"], -32002);
    client.send(&json!({"jsonrpc":"2.0","id":5,"method":"initialize","params":null}));
    assert_eq!(client.receive()["error"]["code"], -32602);
    assert!(client.initialize()["result"]["capabilities"].is_object());
    client.shutdown();
}

#[test]
fn rejects_requests_when_shutdown_was_already_acknowledged() {
    let mut client = Client::spawn();
    client.initialize();
    client.send(&json!({"jsonrpc":"2.0","id":6,"method":"shutdown","params":{}}));
    assert_eq!(client.receive()["error"]["code"], -32602);
    client.send(&json!({"jsonrpc":"2.0","id":7,"method":"shutdown","params":null}));
    assert_eq!(client.receive(), json!({"id":7,"result":null}));
    client.send(&json!({"jsonrpc":"2.0","id":8,"method":"textDocument/completion","params":{}}));
    assert_eq!(client.receive()["error"]["code"], -32600);
    client.exit(0);
}

#[test]
fn clears_live_errors_when_file_is_closed_then_reopened() {
    let mut client = Client::spawn();
    client.initialize();
    let uri = "file:///tmp/iris-lsp-memory-only.iris";
    assert_eq!(
        client.open(uri, "\\")["params"]["diagnostics"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    client.send(&json!({"jsonrpc":"2.0","method":"textDocument/didClose",
        "params":{"textDocument":{"uri":uri}}}));
    let closed = client.receive();
    assert_eq!(closed["params"]["uri"], uri);
    assert_eq!(closed["params"]["diagnostics"], json!([]));
    let reopened = client.open(uri, "let clean = 1");
    assert_eq!(reopened["params"]["version"], 1);
    assert_eq!(reopened["params"]["diagnostics"], json!([]));
    client.shutdown();
}
