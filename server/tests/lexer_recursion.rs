mod support;

use serde_json::{Value, json};
use support::Client;

fn hostile_source() -> String {
    let source = format!("let x={}\"x\"{}", "\"${".repeat(10000), "}\"".repeat(10000));
    assert_eq!(source.len(), 50009);
    source
}

fn assert_resource_limit(client: &Client, publication: &Value, version: i32) {
    assert_eq!(publication["method"], "textDocument/publishDiagnostics");
    assert_eq!(publication["params"]["uri"], "untitled:lexer-recursion");
    assert_eq!(publication["params"]["version"], version);
    assert_eq!(publication["params"]["diagnostics"], json!([]));
    let log = client.receive();
    assert_eq!(log["method"], "window/logMessage");
    let event: Value = serde_json::from_str(log["params"]["message"].as_str().unwrap()).unwrap();
    assert_eq!(
        event,
        json!({"event":"lexer.unlocated", "code":"LEX_RESOURCE_LIMIT",
        "uri":"untitled:lexer-recursion", "version":version})
    );
}

fn assert_completion(client: &mut Client) {
    client.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion",
        "params":{"textDocument":{"uri":"untitled:lexer-recursion"},
            "position":{"line":0,"character":0}}}),
    );
    let response = client.receive();
    assert_eq!(response["id"], 2);
    let labels: Vec<_> = response["result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["label"].as_str().unwrap())
        .collect();
    let expected: Vec<String> =
        serde_json::from_str(include_str!("../../language/keywords.json")).unwrap();
    assert_eq!(labels, expected);
    client.send(&json!({"jsonrpc":"2.0","id":3,"method":"iris/unknown","params":{}}));
    let unknown = client.response();
    assert_eq!(unknown["id"], 3);
    assert_eq!(unknown["error"]["code"], -32601);
}

#[test]
fn remains_responsive_when_did_open_contains_deep_interpolation() {
    let mut client = Client::spawn();
    client.initialize();
    let source = hostile_source();

    let publication = client.open("untitled:lexer-recursion", &source);

    assert_resource_limit(&client, &publication, 1);
    assert_completion(&mut client);
    client.shutdown();
}

#[test]
fn remains_responsive_when_did_change_contains_deep_interpolation() {
    let mut client = Client::spawn();
    client.initialize();
    assert_eq!(
        client.open("untitled:lexer-recursion", "let x=1")["params"]["diagnostics"],
        json!([])
    );
    let source = hostile_source();

    client.send(
        &json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
        "textDocument":{"uri":"untitled:lexer-recursion","version":2},
        "contentChanges":[{"text":source}]}}),
    );

    assert_resource_limit(&client, &client.receive(), 2);
    assert_completion(&mut client);
    client.send(
        &json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
        "textDocument":{"uri":"untitled:lexer-recursion","version":3},
        "contentChanges":[{"text":"let x=1"}]}}),
    );
    let repaired = client.receive();
    assert_eq!(repaired["params"]["version"], 3);
    assert_eq!(repaired["params"]["diagnostics"], json!([]));
    assert_completion(&mut client);
    client.shutdown();
}
