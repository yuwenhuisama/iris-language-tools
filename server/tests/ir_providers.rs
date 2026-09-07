mod support;

use serde_json::{Value, json};
use support::Client;

const URI: &str = "file:///iris-provider-tests/example.ir";

#[test]
fn separates_outer_and_shadow_references_when_ir_buffer_has_repeated_names() {
    let mut given = Client::spawn();
    let capabilities = given.initialize()["result"]["capabilities"].clone();
    assert_eq!(capabilities["referencesProvider"], true);
    let source = "module Main { let value = 1; if true { let value = value; value }; value }";
    given.open(URI, source);
    let outer = source.find("value").unwrap();
    let inner = source.find("value = value").unwrap();
    let initializer = source.find("= value").unwrap() + 2;
    let inner_use = source.find("value }").unwrap();
    let outer_use = source.rfind("value").unwrap();

    for (position, uses) in [
        (outer, vec![initializer, outer_use]),
        (inner, vec![inner_use]),
    ] {
        for include in [false, true] {
            given.send(
                &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{
                "textDocument":{"uri":URI},"position":{"line":0,"character":position},
                "context":{"includeDeclaration":include}}}),
            );
            let when = given.response();

            let mut expected = uses.clone();
            if include {
                expected.insert(0, position);
            }
            let locations: Vec<Value> = expected
                .into_iter()
                .map(|start| {
                    json!({
                "uri":URI,"range":{"start":{"line":0,"character":start},
                "end":{"line":0,"character":start+5}}})
                })
                .collect();
            assert_eq!(when["result"], json!(locations));
        }
    }
    given.shutdown();
}

#[test]
fn completes_source_members_when_ir_buffer_has_prefix_or_trailing_dot() {
    let mut given = Client::spawn();
    let capabilities = given.initialize()["result"]["capabilities"].clone();
    assert_eq!(
        capabilities["completionProvider"]["triggerCharacters"],
        json!([".", ":"])
    );
    for (suffix, prefix, incomplete) in [("re }", 2, false), ("", 0, true)] {
        let source = format!(
            "class Box {{ public fun read() {{}} }} module Main {{ let item = Box.new(); item.{suffix}"
        );
        given.open(URI, &source);
        let start = source.rfind("item.").unwrap() + 5;

        given.send(
            &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{
            "textDocument":{"uri":URI},"position":{"line":0,"character":start+prefix}}}),
        );
        let when = given.response();

        assert_eq!(when["result"]["isIncomplete"], incomplete);
        let items = when["result"]["items"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["label"], "read");
        assert_eq!(items[0]["kind"], 2);
        assert_eq!(
            items[0]["textEdit"],
            json!({"newText":"read","range":{
            "start":{"line":0,"character":start},"end":{"line":0,"character":start+prefix}}})
        );
        given.send(&json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":URI}}}));
        assert_eq!(given.receive()["params"]["diagnostics"], json!([]));
    }
    given.shutdown();
}

#[test]
fn returns_expected_formatting_edit_when_ir_buffer_is_unformatted() {
    let mut given = Client::spawn();
    let capabilities = given.initialize()["result"]["capabilities"].clone();
    assert_eq!(capabilities["documentFormattingProvider"], true);
    given.open(URI, "fun latest() {\nprint(2)\n}");

    given.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/formatting","params":{
        "textDocument":{"uri":URI},"options":{"tabSize":2,"insertSpaces":true}}}),
    );
    let when = given.response();

    assert_eq!(
        when["result"],
        json!([{"range":{
        "start":{"line":0,"character":0},"end":{"line":2,"character":1}},
        "newText":"fun latest() {\n  print(2)\n}\n"}])
    );
    given.shutdown();
}
