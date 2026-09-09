mod support;

use serde_json::json;
use support::Client;

#[test]
fn reports_element_types_when_split_is_indexed_over_stdio() {
    let text = "let a=\"ffff\".split(\"\");let b=a[0];";
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:array-elements", text);

    given.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/inlayHint",
        "params":{"textDocument":{"uri":"untitled:array-elements"},"range":{
            "start":{"line":0,"character":0},"end":{"line":0,"character":text.len()}}}}),
    );
    let when = given.response();

    assert_eq!(
        when["result"],
        json!([
            {"position":{"line":0,"character":5},"label":": Array<String>","kind":1},
            {"position":{"line":0,"character":28},"label":": String?","kind":1}
        ])
    );
    given.shutdown();
}

#[test]
fn reports_hover_types_when_split_bindings_are_queried_over_stdio() {
    for (character, expected) in [(4, "Array<String>"), (27, "String?")] {
        let mut given = Client::spawn();
        given.initialize();
        given.open(
            "untitled:array-hover",
            "let a=\"ffff\".split(\"\");let b=a[0];",
        );

        given.send(
            &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover",
            "params":{"textDocument":{"uri":"untitled:array-hover"},
                "position":{"line":0,"character":character}}}),
        );
        let when = given.response();

        assert!(
            when["result"]["contents"]["value"]
                .as_str()
                .unwrap()
                .contains(expected),
            "{when}"
        );
        given.shutdown();
    }
}

#[test]
fn suppresses_string_members_when_index_result_is_nullable_over_stdio() {
    let text = "let a=\"ffff\".split(\"\");let b=a[0]; b.";
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:array-completion", text);

    given.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion",
        "params":{"textDocument":{"uri":"untitled:array-completion"},
            "position":{"line":0,"character":text.len()}}}),
    );
    let when = given.response();

    assert_eq!(when["result"]["items"], json!([]));
    given.shutdown();
}
