mod support;

use serde_json::{Value, json};
use support::Client;

fn hover(client: &mut Client, uri: &str, position: (u32, u32)) -> Value {
    client.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover",
        "params":{"textDocument":{"uri":uri},
        "position":{"line":position.0,"character":position.1}}}),
    );
    client.response()
}

#[test]
fn advertises_hover_when_initialized() {
    let mut given = Client::spawn();

    let when = given.initialize();

    assert_eq!(when["result"]["capabilities"]["hoverProvider"], true);
    given.shutdown();
}

#[test]
fn returns_plaintext_use_range_when_client_capability_is_missing() {
    let mut given = Client::spawn();
    given.initialize();
    given.open(
        "untitled:hover",
        "module Main {\r\n let value = 1\r\n '\u{1f600}'; value\r\n}",
    );

    let when = hover(&mut given, "untitled:hover", (2, 8));

    assert_eq!(
        when,
        json!({"id":2,"result":{
            "contents":{"kind":"plaintext","value":"let value: Integer"},
            "range":{"start":{"line":2,"character":7},"end":{"line":2,"character":12}}
        }})
    );
    given.shutdown();
}

#[test]
fn resolves_headers_when_hovering_source_class_and_typed_method() {
    let mut given = Client::spawn();
    given.initialize();
    let text = "class Box { public fun read() -> Integer { 1 } }\nmodule Main { let item = Box.new(); item.read() }";
    given.open("untitled:hover", text);
    for (column, signature) in [(25, "class Box"), (42, "public fun read() -> Integer")] {
        let when = hover(&mut given, "untitled:hover", (1, column));

        assert_eq!(when["result"]["contents"]["value"], signature, "{when}");
    }
    given.shutdown();
}

#[test]
fn returns_null_when_source_symbol_or_syntax_is_unknown() {
    let mut given = Client::spawn();
    given.initialize();
    for (uri, text, column) in [
        ("untitled:unknown", None, 0),
        ("untitled:name", Some("module Main { missing }"), 15),
        (
            "untitled:syntax",
            Some("module Main { let value = 1; let broken = ; value }"),
            43,
        ),
    ] {
        if let Some(text) = text {
            given.open(uri, text);
        }

        let when = hover(&mut given, uri, (0, column));

        assert_eq!(when, json!({"id":2,"result":null}));
    }
    given.shutdown();
}

#[test]
fn rejects_params_when_hover_position_is_malformed() {
    let mut given = Client::spawn();
    given.initialize();
    for params in [
        json!(null),
        json!({"textDocument":{"uri":"untitled:hover"}}),
        json!({"textDocument":{"uri":"untitled:hover"},"position":{"line":-1,"character":0}}),
    ] {
        given.send(&json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":params}));

        let when = given.response();

        assert_eq!(when["error"]["code"], -32602);
    }
    given.shutdown();
}

#[test]
fn rejects_position_when_it_splits_an_astral_scalar() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:hover", "module Main { '\u{1f600}' }");

    let when = hover(&mut given, "untitled:hover", (0, 16));

    assert_eq!(when["error"]["code"], -32602);
    given.shutdown();
}
