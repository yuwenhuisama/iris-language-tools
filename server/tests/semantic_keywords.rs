mod support;

use serde_json::json;
use support::Client;

#[test]
fn retains_keywords_when_cursor_is_in_ordinary_context() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:keywords", "let value = 1");
    given.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{
        "textDocument":{"uri":"untitled:keywords"},"position":{"line":0,"character":0}}}),
    );
    let when = given.response();
    let items = when["result"]["items"].as_array().unwrap();
    let keywords: Vec<_> = items
        .iter()
        .filter(|item| item["kind"] == 14)
        .map(|item| item["label"].as_str().unwrap())
        .collect();
    let expected: Vec<String> =
        serde_json::from_str(include_str!("../../language/keywords.json")).unwrap();
    assert_eq!(keywords.len(), 50);
    assert_eq!(keywords, expected);
    assert!(
        items
            .iter()
            .any(|item| item["label"] == "String" && item["kind"] == 7)
    );
    assert_eq!(when["result"]["isIncomplete"], false);
    given.shutdown();
}

#[test]
fn suppresses_keywords_when_cursor_is_in_protected_text() {
    let mut given = Client::spawn();
    given.initialize();
    let cases = [
        "module Main { let value = 'vis",
        "// comment",
        "module Main { let value = 123",
    ];
    for (index, text) in cases.iter().enumerate() {
        let uri = format!("untitled:protected{index}");
        given.open(&uri, text);
        given.send(
            &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{
            "textDocument":{"uri":uri},"position":{"line":0,"character":text.len()}}}),
        );
        let when = given.response();
        let items = when["result"]
            .as_array()
            .or_else(|| when["result"]["items"].as_array())
            .unwrap();
        assert!(items.is_empty(), "{when:?}");
    }
    given.shutdown();
}

#[test]
fn suppresses_keywords_when_receiver_or_namespace_is_unknown() {
    let mut given = Client::spawn();
    given.initialize();
    for (index, text) in ["unknown.", "unknown.re", "Unknown::", "Unknown::Th"]
        .iter()
        .enumerate()
    {
        let uri = format!("untitled:unknown{index}");
        given.open(&uri, text);
        given.send(
            &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{
            "textDocument":{"uri":uri},"position":{"line":0,"character":text.len()}}}),
        );
        let when = given.response();
        assert_eq!(when["result"]["items"], json!([]), "{when:?}");
    }
    given.shutdown();
}

#[test]
fn merges_keywords_once_when_symbols_are_available() {
    let mut given = Client::spawn();
    given.initialize();
    let text = "module Main { let value = 1; va }";
    given.open("untitled:mixed", text);
    given.send(&json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{
        "textDocument":{"uri":"untitled:mixed"},"position":{"line":0,"character":text.find("va }").unwrap()+2}}}));
    let when = given.response();
    let items = when["result"]["items"].as_array().unwrap();
    assert_eq!(items.iter().filter(|item| item["kind"] == 14).count(), 50);
    assert_eq!(
        items.iter().filter(|item| item["label"] == "value").count(),
        1
    );
    assert_eq!(when["result"]["isIncomplete"], false);
    given.shutdown();
}
