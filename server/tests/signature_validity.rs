mod support;

use iris_formatter::{FormatOutcome, SkipReason, format_document};
use serde_json::{Value, json};
use support::Client;

fn query(client: &mut Client, method: &str, source: (&str, usize)) -> Value {
    let prefix = &source.0[..source.1];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count();
    let character = prefix.rsplit('\n').next().unwrap().encode_utf16().count();
    client.send(&json!({"jsonrpc":"2.0","id":2,"method":method,
        "params":{"textDocument":{"uri":"untitled:signature-validity"},
        "position":{"line":line,"character":character}}}));
    client.response()
}

#[test]
fn queries_return_null_when_strict_parser_rejects_method_header() {
    let mut given = Client::spawn();
    given.initialize();
    for header in [
        "read(value, key option, second)",
        "read(value, key option, second) -> String",
        "read(value, *first, *second)",
        "read(value, **first, **second)",
        "read(value, &first, &second)",
        "read<>(value, second)",
        "read(value = , second)",
    ] {
        for body in [" {} ", "\n{}\n", "\n"] {
            let complete = format!("module Main {{ fun {header}{body}read(1, 2) }}");
            assert_eq!(
                format_document(&complete),
                FormatOutcome::Skipped(SkipReason::ParseDiagnostics),
                "strict parser must reject: {complete}"
            );
            let text = format!("module Main {{ fun {header}{body}read(1,");
            given.open("untitled:signature-validity", &text);

            for (method, byte) in [
                ("textDocument/hover", text.find("read").unwrap()),
                ("textDocument/hover", text.rfind("read").unwrap()),
                ("textDocument/signatureHelp", text.len()),
            ] {
                let when = query(&mut given, method, (&text, byte));

                assert_eq!(when, json!({"id":2,"result":null}), "{method}: {text}");
            }
            given.send(&json!({"jsonrpc":"2.0","method":"textDocument/didClose",
                "params":{"textDocument":{"uri":"untitled:signature-validity"}}}));
            given.receive();
        }
    }
    given.shutdown();
}

#[test]
fn queries_remain_usable_when_valid_header_has_incomplete_body_and_call() {
    let mut given = Client::spawn();
    given.initialize();
    for text in [
        "module Main { fun read(value, second) {} read(1,",
        "module Main { fun read(value, second) { read(1,",
        "module Main { fun read(_, _) { read(1,",
    ] {
        given.open("untitled:signature-validity", text);

        for byte in [text.find("read").unwrap(), text.rfind("read").unwrap()] {
            let when = query(&mut given, "textDocument/hover", (text, byte));

            assert!(when["result"].is_object(), "{when}: {text}");
        }
        let when = query(&mut given, "textDocument/signatureHelp", (text, text.len()));

        assert_eq!(when["result"]["activeParameter"], 1, "{when}: {text}");
        assert_eq!(
            when["result"]["signatures"][0]["parameters"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        given.send(&json!({"jsonrpc":"2.0","method":"textDocument/didClose",
            "params":{"textDocument":{"uri":"untitled:signature-validity"}}}));
        given.receive();
    }
    given.shutdown();
}
