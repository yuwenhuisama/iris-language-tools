mod support;

use serde_json::{Value, json};
use support::Client;

fn signature(client: &mut Client, text: &str, byte: usize) -> Value {
    let prefix = &text[..byte];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let character = prefix.rsplit('\n').next().unwrap().encode_utf16().count();
    client.send(&json!({"jsonrpc":"2.0","id":2,"method":"textDocument/signatureHelp",
        "params":{"textDocument":{"uri":"untitled:signature"},"position":{"line":line,"character":character},
        "context":{"triggerKind":2,"triggerCharacter":",","isRetrigger":true,
        "activeSignatureHelp":{"signatures":[{"label":"untrusted"}],"activeSignature":99,"activeParameter":99}}}}));
    client.response()
}

#[test]
fn advertises_triggers_when_initialized() {
    let mut given = Client::spawn();

    let when = given.initialize();

    assert_eq!(
        when["result"]["capabilities"]["signatureHelpProvider"],
        json!({
            "triggerCharacters":["(",",",":"],"retriggerCharacters":[")"]
        })
    );
    given.shutdown();
}

#[test]
fn returns_substrings_when_offset_support_is_absent() {
    let mut given = Client::spawn();
    given.initialize();
    let text = "module Main { public fun read(_, value = '\u{1f600}', *rest, key option, **kwargs) {} read(1, 2, 3) }";
    given.open("untitled:signature", text);

    let when = signature(&mut given, text, text.rfind("3)").unwrap());

    assert_eq!(when["result"]["activeSignature"], 0);
    assert_eq!(when["result"]["activeParameter"], 2);
    let info = &when["result"]["signatures"][0];
    assert_eq!(
        info["label"],
        "public fun read(_: Dynamic<Object>, value: Dynamic<Object> = '\u{1f600}', *rest: Dynamic<Object>, key option: Dynamic<Object>, **kwargs: Dynamic<Object>) -> Dynamic<Object>"
    );
    assert_eq!(info["parameters"][2]["label"], "*rest: Dynamic<Object>");
    assert!(info.get("activeParameter").is_none());
    assert!(when["result"].get("textEdits").is_none());
    given.shutdown();
}

#[test]
fn rejects_malformed_params_when_context_or_position_is_invalid() {
    let mut given = Client::spawn();
    given.initialize();
    for params in [
        json!(null),
        json!({"textDocument":{"uri":"untitled:signature"}}),
        json!({"textDocument":{"uri":"untitled:signature"},"position":{"line":-1,"character":0}}),
        json!({"textDocument":{"uri":"untitled:signature"},"position":{"line":0,"character":0},"context":{"triggerKind":2,"isRetrigger":"yes"}}),
        json!({"textDocument":{"uri":"untitled:signature"},"position":{"line":0,"character":0},"context":{"triggerKind":2,"isRetrigger":true,"activeSignatureHelp":{"signatures":"bad"}}}),
    ] {
        given.send(
            &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/signatureHelp","params":params}),
        );

        let when = given.response();

        assert_eq!(when["error"]["code"], -32602);
    }
    given.shutdown();
}

#[test]
fn maps_slots_when_nested_commas_keywords_and_incomplete_calls_occur() {
    let mut given = Client::spawn();
    given.initialize();
    for (call, active) in [
        ("read(", 0),
        ("read(1,", 1),
        ("read(option:", 3),
        ("read(other: 1", 4),
        ("read([1,2], 'a,b', /* , */ 3", 2),
        ("read(inner(1,2),", 1),
        ("read(1, inner(", 0),
    ] {
        let text = format!(
            "module Main {{ public fun read(_, value = 1, *rest, key option, **kwargs) {{}} public fun inner(a,b) {{}} {call}"
        );
        given.open("untitled:signature", &text);

        let when = signature(&mut given, &text, text.len());

        assert_eq!(when["result"]["activeParameter"], active, "{call}: {when}");
        let expected = if call.ends_with("inner(") {
            "inner("
        } else {
            "read("
        };
        assert!(
            when["result"]["signatures"][0]["label"]
                .as_str()
                .unwrap()
                .contains(expected)
        );
        given.send(&json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":"untitled:signature"}}}));
        given.receive();
    }
    given.shutdown();
}

#[test]
fn returns_null_when_callee_or_argument_mapping_is_unknown() {
    let mut given = Client::spawn();
    given.initialize();
    assert_eq!(signature(&mut given, "", 0), json!({"id":2,"result":null}));
    for call in [
        "unknown(",
        "read(1, unknown(",
        "read(other: 1",
        "read(1, 2",
        "read(1)",
    ] {
        let text = format!("module Main {{ public fun read(value) {{}} {call}");
        given.open("untitled:signature", &text);

        let when = signature(&mut given, &text, text.len());

        assert_eq!(when, json!({"id":2,"result":null}), "{call}");
        given.send(&json!({"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":"untitled:signature"}}}));
        given.receive();
    }
    given.shutdown();
}

#[test]
fn omits_active_parameter_when_zero_argument_call_has_existing_closer() {
    let mut given = Client::spawn();
    given.initialize();
    let text = "module Main { public fun read() {} read() }";
    given.open("untitled:signature", text);

    let when = signature(&mut given, text, text.rfind(')').unwrap());

    assert_eq!(when["result"]["activeSignature"], 0);
    assert!(when["result"].get("activeParameter").is_none());
    assert_eq!(when["result"]["signatures"][0]["parameters"], json!([]));
    given.shutdown();
}

#[test]
fn selects_next_slot_when_trailing_comma_precedes_existing_closer() {
    let mut given = Client::spawn();
    given.initialize();
    let text = "module Main { public fun read(first, second) {} read(1,) }";
    given.open("untitled:signature", text);

    let when = signature(&mut given, text, text.rfind(')').unwrap());

    assert_eq!(when["result"]["activeParameter"], 1, "{when}");
    given.shutdown();
}

#[test]
fn rejects_position_when_it_splits_a_surrogate_pair() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:signature", "module Main { '\u{1f600}' }");
    given.send(&json!({"jsonrpc":"2.0","id":2,"method":"textDocument/signatureHelp",
        "params":{"textDocument":{"uri":"untitled:signature"},"position":{"line":0,"character":16}}}));

    let when = given.response();

    assert_eq!(when["error"]["code"], -32602);
    given.shutdown();
}
