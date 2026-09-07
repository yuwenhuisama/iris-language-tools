mod support;

use serde_json::{Value, json};
use support::Client;

fn query(client: &mut Client, position: Value) -> Value {
    let mut params = json!({"textDocument":{"uri":"untitled:edge"}});
    params["position"] = position;
    client
        .send(&json!({"jsonrpc":"2.0","id":2,"method":"textDocument/definition", "params":params}));
    client.response()
}

#[test]
fn maps_utf16_when_definition_follows_astral_literal() {
    let mut given = Client::spawn();
    given.initialize();
    let text = "module Main { let text = '\u{1f600}'; let value = 1; value }";
    given.open("untitled:edge", text);
    let name = text.find("value").unwrap();
    let use_site = text.rfind("value").unwrap();
    let column = text[..name].encode_utf16().count();
    let when = query(
        &mut given,
        json!({"line":0,"character":text[..use_site].encode_utf16().count()}),
    );
    assert_eq!(
        when["result"][0]["range"],
        json!({"start":{"line":0,"character":column},
        "end":{"line":0,"character":column+5}})
    );
    given.shutdown();
}

#[test]
fn rejects_position_when_line_invalid_or_utf16_splits_surrogate() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:edge", "// \u{1f600}\nmodule Main {}");
    for position in [
        json!({"line":5,"character":0}),
        json!({"line":0,"character":4}),
    ] {
        let when = query(&mut given, position);
        assert_eq!(when["error"]["code"], -32602);
    }
    given.shutdown();
}

#[test]
fn clamps_past_eol_when_position_exceeds_last_column() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:edge", "module Main { let local = 1; local\n}");
    let when = query(&mut given, json!({"line":0,"character":1000}));
    assert_eq!(when["result"][0]["range"]["start"]["character"], 18);
    given.shutdown();
}

#[test]
fn resolves_inner_parameter_when_outer_binding_has_same_name() {
    let mut given = Client::spawn();
    given.initialize();
    let text = "module Main { let value = 1; fun read(value) { value } }";
    given.open("untitled:edge", text);
    let when = query(
        &mut given,
        json!({"line":0,"character":text.rfind("value").unwrap()}),
    );
    assert_eq!(
        when["result"][0]["range"]["start"]["character"],
        text.find("value)").unwrap()
    );
    given.shutdown();
}

#[test]
fn rejects_malformed_parameters_when_each_semantic_provider_is_called() {
    let mut given = Client::spawn();
    given.initialize();
    for method in [
        "textDocument/definition",
        "textDocument/references",
        "textDocument/completion",
        "textDocument/inlayHint",
    ] {
        given.send(&json!({"jsonrpc":"2.0","id":2,"method":method,"params":{}}));
        let when = given.response();
        assert_eq!(when["error"]["code"], -32602);
    }
    given.shutdown();
}

#[test]
fn includes_declaration_when_references_explicitly_requests_it() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:edge", "module Main { let local = 1; local }");
    given.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{
        "textDocument":{"uri":"untitled:edge"},"position":{"line":0,"character":20},
        "context":{"includeDeclaration":true}}}),
    );
    let when = given.response();
    let starts: Vec<_> = when["result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|site| site["range"]["start"]["character"].as_u64().unwrap())
        .collect();
    assert_eq!(starts, [18, 29]);
    given.shutdown();
}

#[test]
fn keeps_known_members_when_typing_trailing_dot_with_missing_brace() {
    let mut given = Client::spawn();
    given.initialize();
    let text = "class Box { public fun read() {} } module Main { let item = Box.new(); item.";
    given.open("untitled:edge", text);
    given.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{
        "textDocument":{"uri":"untitled:edge"},"position":{"line":0,"character":text.len()}}}),
    );
    let when = given.response();
    assert_eq!(when["result"]["isIncomplete"], true);
    assert_eq!(when["result"]["items"][0]["label"], "read");
    given.shutdown();
}

#[test]
fn forgets_closed_document_when_reopened_at_same_version() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:edge", "module Main { let local = 1; local }");
    assert_eq!(
        query(&mut given, json!({"line":0,"character":30}))["result"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    given.send(&json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":"untitled:edge"}}}));
    assert_eq!(given.receive()["params"]["diagnostics"], json!([]));
    assert_eq!(
        query(&mut given, json!({"line":0,"character":30}))["result"],
        json!([])
    );
    given.open("untitled:edge", "module Main { local }");
    let when = query(&mut given, json!({"line":0,"character":15}));
    assert_eq!(when["result"], json!([]));
    given.shutdown();
}

#[test]
fn answers_once_when_client_cancels_a_real_semantic_request() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:edge", "module Main { let local = 1; local }");
    given.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{
        "textDocument":{"uri":"untitled:edge"},"position":{"line":0,"character":20},
        "context":{"includeDeclaration":true}}}),
    );
    given.send(&json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":2}}));
    let when = given.response();
    assert_eq!(when["id"], 2);
    match when.get("error") {
        Some(error) => assert_eq!(error["code"], -32800),
        None => assert_eq!(when["result"].as_array().unwrap().len(), 2),
    }
    given.shutdown();
}
