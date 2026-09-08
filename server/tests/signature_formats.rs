mod support;

use serde_json::{Value, json};
use support::Client;

fn client(settings: &Value) -> Client {
    let mut client = Client::spawn();
    client.send(
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "capabilities":{"textDocument":{"hover":{"contentFormat":["markdown"]},
        "signatureHelp":{"signatureInformation":settings}}}}}),
    );
    assert!(client.receive()["error"].is_null());
    client.send(&json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
    client
}

fn signature(client: &mut Client, source: &str) -> Value {
    client.send(&json!({"jsonrpc":"2.0","id":2,"method":"textDocument/signatureHelp","params":{
        "textDocument":{"uri":"untitled:formats"},"position":{"line":3,"character":source.lines().nth(3).unwrap().encode_utf16().count()}}}));
    client.response()
}

const SOURCE: &str = "module Main {\n/// <img> [run](command:bad) & `code`\npublic fun read(first = '\u{1f600}', second = 2) {}\nread(1,";

#[test]
fn returns_utf16_offsets_when_client_explicitly_supports_them() {
    let mut given = client(&json!({"documentationFormat":["markdown","plaintext"],
        "parameterInformation":{"labelOffsetSupport":true},"activeParameterSupport":true}));
    given.open("untitled:formats", SOURCE);

    let when = signature(&mut given, SOURCE);

    let info = &when["result"]["signatures"][0];
    let label: Vec<_> = info["label"].as_str().unwrap().encode_utf16().collect();
    for (parameter, expected) in info["parameters"].as_array().unwrap().iter().zip([
        "first: Dynamic<Object> = '\u{1f600}'",
        "second: Dynamic<Object> = 2",
    ]) {
        let offsets = parameter["label"].as_array().unwrap();
        let start = usize::try_from(offsets[0].as_u64().unwrap()).unwrap();
        let end = usize::try_from(offsets[1].as_u64().unwrap()).unwrap();
        assert_eq!(String::from_utf16(&label[start..end]).unwrap(), expected);
    }
    assert_eq!(when["result"]["activeParameter"], 1);
    assert_eq!(
        info["documentation"],
        json!({"kind":"markdown","value":"&lt;img&gt; \\[run\\]\\(command\\:bad\\) &amp; \\`code\\`"})
    );
    given.shutdown();
}

#[test]
fn keeps_plaintext_independent_of_hover_when_preferences_are_missing_empty_or_plain_first() {
    for settings in [
        json!({}),
        json!({"documentationFormat":[]}),
        json!({"documentationFormat":["plaintext","markdown"],"parameterInformation":{"labelOffsetSupport":false}}),
    ] {
        let mut given = client(&settings);
        given.open("untitled:formats", SOURCE);

        let when = signature(&mut given, SOURCE);

        let info = &when["result"]["signatures"][0];
        assert_eq!(
            info["documentation"],
            json!({"kind":"plaintext","value":"<img> [run](command:bad) & `code`"})
        );
        assert_eq!(
            info["parameters"][1]["label"],
            "second: Dynamic<Object> = 2"
        );
        assert!(info.get("activeParameter").is_none());
        given.shutdown();
    }
}

#[test]
fn leaves_document_unchanged_when_signature_help_is_requested_repeatedly() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:formats", SOURCE);
    let before = signature(&mut given, SOURCE);

    let when = signature(&mut given, SOURCE);

    assert_eq!(when, before);
    assert!(when["result"].get("textEdit").is_none());
    given.shutdown();
}
