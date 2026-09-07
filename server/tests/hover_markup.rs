mod support;

use serde_json::{Value, json};
use support::Client;

fn initialized(formats: &[&str]) -> Client {
    let mut client = Client::spawn();
    client.send(
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "capabilities":{"textDocument":{"hover":{"contentFormat":formats}}}}}),
    );
    assert_eq!(
        client.receive()["result"]["capabilities"]["hoverProvider"],
        true
    );
    client.send(&json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
    client
}

fn hover(client: &mut Client) -> Value {
    client.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{
        "textDocument":{"uri":"untitled:hover"},"position":{"line":0,"character":19}}}),
    );
    client.response()["result"]["contents"].clone()
}

#[test]
fn escapes_markdown_when_signature_contains_default_string_markup() {
    let mut given = initialized(&["markdown", "plaintext"]);
    given.open(
        "untitled:hover",
        "module Main { fun read(value = '[go](command:run) <b>& ```') {} }",
    );

    let when = hover(&mut given);

    assert_eq!(
        when,
        json!({"kind":"markdown","value":
        "private fun read\\(value\\: Dynamic\\<Object\\> \\= \\'\\[go\\]\\(command\\:run\\) \\<b\\>\\& \\`\\`\\`\\'\\) \\-\\> Dynamic\\<Object\\>"})
    );
    given.shutdown();
}

#[test]
fn preserves_default_string_when_plaintext_is_preferred() {
    let mut given = initialized(&["plaintext", "markdown"]);
    given.open(
        "untitled:hover",
        "module Main { fun read(value = 'a  b `[go](command:run)` <b>') {} }",
    );

    let when = hover(&mut given);

    assert_eq!(
        when,
        json!({"kind":"plaintext","value":
        "private fun read(value: Dynamic<Object> = 'a  b `[go](command:run)` <b>') -> Dynamic<Object>"})
    );
    given.shutdown();
}

#[test]
fn uses_plaintext_when_format_preference_is_empty() {
    let mut given = initialized(&[]);
    given.open("untitled:hover", "module Main { fun read() {} }");

    let when = hover(&mut given);

    assert_eq!(
        when,
        json!({"kind":"plaintext","value":"private fun read() -> Dynamic<Object>"})
    );
    given.shutdown();
}

#[test]
fn bounds_escaped_content_when_default_has_large_backtick_run() {
    let mut given = initialized(&["markdown"]);
    given.open(
        "untitled:hover",
        &format!(
            "module Main {{ fun read(value = '{}') {{}} }}",
            "`".repeat(12_000)
        ),
    );

    let when = hover(&mut given);

    let value = when["value"].as_str().unwrap();
    assert_eq!(when["kind"], "markdown");
    assert!(value.len() <= 8192);
    assert!(value.contains("\\`\\`\\`"));
    assert!(!value.contains("```"));
    given.shutdown();
}

#[test]
fn defaults_to_plaintext_when_method_hover_capability_is_missing() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:hover", "module Main { fun read() {} }");

    let when = hover(&mut given);

    assert_eq!(when["kind"], "plaintext");
    given.shutdown();
}
