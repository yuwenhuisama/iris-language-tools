mod support;

use serde_json::{Value, json};
use support::Client;

const URI: &str = "untitled:formatting";

fn request(client: &mut Client, options: Value) -> Value {
    let mut message = json!({"jsonrpc":"2.0","id":2,"method":"textDocument/formatting",
        "params":{"textDocument":{"uri":URI}}});
    message["params"]["options"] = options;
    client.send(&message);
    client.response()
}

#[test]
fn advertises_formatting_when_initialized() {
    let mut client = Client::spawn();

    let response = client.initialize();

    assert_eq!(
        response["result"]["capabilities"]["documentFormattingProvider"],
        true
    );
    client.shutdown();
}

#[test]
fn formats_latest_unsaved_text_when_stale_and_equal_changes_follow() {
    let mut client = Client::spawn();
    client.initialize();
    client.open(URI, "fun old() {\nprint(1)\n}");
    client.send(
        &json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
        "textDocument":{"uri":URI,"version":3},
        "contentChanges":[{"text":"fun latest() {\nprint(2)\n}"}]}}),
    );
    assert_eq!(client.receive()["params"]["version"], 3);
    for version in [2, 3] {
        client.send(
            &json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
            "textDocument":{"uri":URI,"version":version},
            "contentChanges":[{"text":"fun stale() {}"}]}}),
        );
    }

    let response = request(&mut client, json!({"tabSize":2,"insertSpaces":true}));

    assert_eq!(
        response["result"],
        json!([{"range":{
        "start":{"line":0,"character":0},"end":{"line":2,"character":1}},
        "newText":"fun latest() {\n  print(2)\n}\n"}])
    );
    client.shutdown();
}

#[test]
fn maps_original_utf16_end_when_source_has_unicode_and_crlf() {
    let mut client = Client::spawn();
    client.initialize();
    client.open(
        URI,
        "\u{feff}fun f() {\r\nprint(\"\u{1f600}\")\r\n} // \u{1f600}",
    );

    let response = request(&mut client, json!({"tabSize":2,"insertSpaces":true}));

    assert_eq!(
        response["result"],
        json!([{"range":{
        "start":{"line":0,"character":0},"end":{"line":2,"character":7}},
        "newText":"fun f() {\n  print(\"\u{1f600}\")\n}  // \u{1f600}\n"}])
    );
    client.shutdown();
}

#[test]
fn uses_official_spaces_when_editor_options_conflict() {
    let mut client = Client::spawn();
    client.initialize();
    client.open(URI, "fun f() {\rprint(\n1\r\n)\r}\r\n");

    let response = request(&mut client, json!({"tabSize":8,"insertSpaces":false}));

    assert_eq!(
        response["result"],
        json!([{"range":{
        "start":{"line":0,"character":0},"end":{"line":5,"character":0}},
        "newText":"fun f() {\n  print(\n    1,\n  )\n}\n"}])
    );
    client.shutdown();
}

#[test]
fn returns_empty_when_document_is_unknown_closed_or_unchanged() {
    let mut client = Client::spawn();
    client.initialize();
    for text in [None, Some("fun f() {\n  print(1)\n}\n")] {
        if let Some(text) = text {
            client.open(URI, text);
        }

        let response = request(&mut client, json!({"tabSize":2,"insertSpaces":true}));

        assert_eq!(response["result"], json!([]));
    }
    client.send(&json!({"jsonrpc":"2.0","method":"textDocument/didClose",
        "params":{"textDocument":{"uri":URI}}}));
    client.receive();

    let response = request(&mut client, json!({"tabSize":2,"insertSpaces":true}));

    assert_eq!(response["result"], json!([]));
    client.shutdown();
}

#[test]
fn returns_invalid_params_when_options_are_invalid() {
    let mut client = Client::spawn();
    client.initialize();
    client.open(URI, "fun f() {\nprint(1)\n}");
    for options in [
        json!({"tabSize":0,"insertSpaces":true}),
        json!({"tabSize":17,"insertSpaces":false}),
        json!({"tabSize":-1,"insertSpaces":true}),
        json!({"tabSize":2.5,"insertSpaces":true}),
        json!({"tabSize":2,"insertSpaces":"yes"}),
        json!({"tabSize":2}),
        json!({"insertSpaces":true}),
        Value::Null,
    ] {
        let response = request(&mut client, options);

        assert_eq!(response["error"]["code"], -32602);
    }
    client.shutdown();
}

#[test]
fn returns_invalid_params_when_document_parameters_are_malformed() {
    let mut client = Client::spawn();
    client.initialize();
    for params in [
        Value::Null,
        json!({}),
        json!({"textDocument":{"uri":42},
        "options":{"tabSize":2,"insertSpaces":true}}),
    ] {
        client.send(
            &json!({"jsonrpc":"2.0","id":3,"method":"textDocument/formatting","params":params}),
        );

        let response = client.receive();

        assert_eq!(response["error"]["code"], -32602);
    }
    client.shutdown();
}

#[test]
fn stays_responsive_when_invalid_or_incomplete_source_is_skipped() {
    let mut client = Client::spawn();
    client.initialize();
    for source in [
        "fun f() {\nprint(1)",
        "fun f() {\n]\n}",
        "fun f() {\n\\\n}",
        "fun f() {\nlet = 1\n}",
    ] {
        client.open(URI, source);

        let response = request(&mut client, json!({"tabSize":2,"insertSpaces":true}));

        assert_eq!(response["result"], json!([]));
        client.send(&json!({"jsonrpc":"2.0","method":"textDocument/didClose",
            "params":{"textDocument":{"uri":URI}}}));
        client.receive();
    }
    client.open(URI, "fun f() {\nprint(1)\n}");

    let response = request(&mut client, json!({"tabSize":2,"insertSpaces":true}));

    assert_eq!(
        response["result"][0]["newText"],
        "fun f() {\n  print(1)\n}\n"
    );
    client.shutdown();
}

#[test]
fn preserves_document_when_formatting_is_requested_twice_without_applying_edits() {
    let mut client = Client::spawn();
    client.initialize();
    client.open(URI, "fun f() {\nprint(1)\n}");
    let first = request(&mut client, json!({"tabSize":2,"insertSpaces":true}));

    let second = request(&mut client, json!({"tabSize":2,"insertSpaces":true}));

    assert_eq!(first["result"][0]["newText"], "fun f() {\n  print(1)\n}\n");
    assert_eq!(second, first);
    client.shutdown();
}

#[test]
fn expands_methods_semicolons_else_and_lists_when_official_style_is_requested() {
    let mut client = Client::spawn();
    client.initialize();
    client.open(
        URI,
        "fun f(){let values=[\n1,\n2\n];if ready {print(values);}else{print(0);}}",
    );

    let response = request(&mut client, json!({"tabSize":8,"insertSpaces":false}));

    assert_eq!(
        response["result"][0]["newText"],
        "fun f() {\n  let values = [\n    1,\n    2,\n  ]\n  if ready {\n    print(values)\n  }\n  else {\n    print(0)\n  }\n}\n"
    );
    client.shutdown();
}
