mod support;

use serde_json::{Value, json};
use support::Client;

fn query(client: &mut Client, method: &str, uri: &str, position: Value) -> Value {
    let mut params = json!({"textDocument":{"uri":uri}});
    params["position"] = position;
    if method == "textDocument/references" {
        params["context"] = json!({"includeDeclaration":false});
    }
    client.send(&json!({"jsonrpc":"2.0","id":2,"method":method,"params":params}));
    client.response()
}

#[test]
fn resolves_definition_when_local_is_used_in_unsaved_source() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:semantic", "module Main { let local = 1; local }");
    let when = query(
        &mut given,
        "textDocument/definition",
        "untitled:semantic",
        json!({"line":0,"character":30}),
    );
    assert_eq!(
        when["result"],
        json!([{"uri":"untitled:semantic", "range":{
        "start":{"line":0,"character":18},"end":{"line":0,"character":23}}}])
    );
    given.shutdown();
}

#[test]
fn excludes_declaration_when_references_context_requests_uses_only() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:semantic", "module Main { let local = 1; local }");
    let when = query(
        &mut given,
        "textDocument/references",
        "untitled:semantic",
        json!({"line":0,"character":20}),
    );
    assert_eq!(
        when["result"],
        json!([{"uri":"untitled:semantic", "range":{
        "start":{"line":0,"character":29},"end":{"line":0,"character":34}}}])
    );
    given.shutdown();
}

#[test]
fn completes_member_when_identifier_is_partial() {
    let mut given = Client::spawn();
    given.initialize();
    let text = "class Box { public fun read() {} } module Main { let item = Box.new(); item.re }";
    given.open("untitled:semantic", text);
    let start = text.rfind("re }").unwrap();
    let when = query(
        &mut given,
        "textDocument/completion",
        "untitled:semantic",
        json!({"line":0,"character":start + 2}),
    );
    assert_eq!(when["result"]["items"][0]["label"], "read");
    assert_eq!(when["result"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        when["result"]["items"][0]["textEdit"]["range"],
        json!({
        "start":{"line":0,"character":start},"end":{"line":0,"character":start+2}})
    );
    given.shutdown();
}

#[test]
fn returns_type_hint_when_requested_range_contains_inferred_binding() {
    let mut given = Client::spawn();
    given.initialize();
    given.open(
        "untitled:semantic",
        "module Main { let local = 1; let other = 2 }",
    );
    given.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/inlayHint",
        "params":{"textDocument":{"uri":"untitled:semantic"},"range":{
        "start":{"line":0,"character":18},"end":{"line":0,"character":24}}}}),
    );
    let when = given.response();
    assert_eq!(
        when["result"],
        json!([{"position":{"line":0,"character":23},
        "label":": Integer","kind":1}])
    );
    given.shutdown();
}
