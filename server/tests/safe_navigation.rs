mod support;

use serde_json::{Value, json};
use support::Client;

const URI: &str = "untitled:safe-navigation";

fn query(client: &mut Client, method: &str, character: usize) -> Value {
    client.send(&json!({"jsonrpc":"2.0","id":2,"method":method,"params":{
        "textDocument":{"uri":URI},"position":{"line":0,"character":character}}}));
    let response = client.response();
    assert!(response.get("error").is_none(), "{response}");
    response["result"].clone()
}

fn hint_labels(client: &mut Client, text: &str) -> Vec<String> {
    client.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/inlayHint",
        "params":{"textDocument":{"uri":URI},"range":{
            "start":{"line":0,"character":0},
            "end":{"line":0,"character":text.len()}}}}),
    );
    let response = client.response();
    assert!(response.get("error").is_none(), "{response}");
    response["result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hint| hint["label"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn reports_optional_types_when_source_method_results_are_safely_chained_over_stdio() {
    let text = "class Box { public fun next() -> Box? { nil } public fun name() -> String { 'x' } } let item = Box.new(); let maybe = item.next()?.name(); let parts = item.next()?.name()?.split('');";
    let mut given = Client::spawn();
    given.initialize();
    given.open(URI, text);

    let when = hint_labels(&mut given, text);
    assert_eq!(
        when,
        [": Box", ": String?", ": Array<String>?"],
        "{text}: {when:?}"
    );

    for (name, expected) in [("maybe =", "String?"), ("parts =", "Array<String>?")] {
        let when = query(&mut given, "textDocument/hover", text.find(name).unwrap());
        assert!(
            when["contents"]["value"]
                .as_str()
                .unwrap()
                .contains(expected),
            "{name}: {when}"
        );
    }
    given.shutdown();
}

#[test]
fn reports_optional_and_asserted_array_types_when_indexed_string_is_chained_over_stdio() {
    let text = "let sections=\"ffff\".split(\"\"); let lines=sections[0]?.split(\"\\n\"); let certain=sections[0]!.split(\"\\n\");";
    let mut given = Client::spawn();
    given.initialize();
    given.open(URI, text);

    let when = hint_labels(&mut given, text);
    assert_eq!(
        when,
        [": Array<String>", ": Array<String>?", ": Array<String>"],
        "{text}: {when:?}"
    );

    for (name, expected) in [("lines=", "Array<String>?"), ("certain=", "Array<String>")] {
        let when = query(&mut given, "textDocument/hover", text.find(name).unwrap());
        assert!(
            when["contents"]["value"]
                .as_str()
                .unwrap()
                .contains(expected),
            "{name}: {when}"
        );
    }
    given.shutdown();
}

#[test]
fn reports_string_type_after_asserting_nullable_index_over_stdio() {
    let text = "let sections='ffff'.split(''); let first=sections[0]!;";
    let mut given = Client::spawn();
    given.initialize();
    given.open(URI, text);

    let when = hint_labels(&mut given, text);
    assert_eq!(when, [": Array<String>", ": String"], "{text}: {when:?}");

    let when = query(
        &mut given,
        "textDocument/hover",
        text.find("first=").unwrap(),
    );
    assert!(
        when["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("String"),
        "{when}"
    );
    given.shutdown();
}

#[test]
fn completes_string_members_after_postfix_non_null_but_not_ordinary_nullable_access() {
    for (suffix, expected) in [
        ("sections[0].", false),
        ("sections[0]!.", true),
        ("sections[0]!.re", true),
    ] {
        let text = format!("let sections='ffff'.split(''); {suffix}");
        let mut given = Client::spawn();
        given.initialize();
        given.open(URI, &text);

        let when = query(&mut given, "textDocument/completion", text.len());
        let labels: Vec<_> = when["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["label"].as_str().unwrap())
            .collect();
        assert_eq!(labels.contains(&"replace"), expected, "{text}: {when}");
        if expected {
            assert!(!labels.contains(&"push"), "{text}: {when}");
        } else {
            assert!(labels.is_empty(), "{text}: {when}");
        }
        given.shutdown();
    }
}
