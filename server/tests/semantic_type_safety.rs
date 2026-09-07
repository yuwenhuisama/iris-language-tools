mod support;

use serde_json::{Value, json};
use support::Client;

const URI: &str = "untitled:type-safety";

struct Scenario {
    text: String,
    copy_type: Option<&'static str>,
    instance: bool,
    omitted: bool,
}

fn scenarios() -> Vec<Scenario> {
    let mut scenarios = Vec::new();
    for (base, header) in [
        ("class", "class Box extends Base {}"),
        ("contract", "class Box for Base {}"),
        ("module", "class Box mixin Base {}"),
        ("module", "module Box mixin Base {}"),
        ("contract", "module Box for Base {}"),
        ("contract", "contract Box extends Base {}"),
    ] {
        scenarios.push(Scenario {
            text: format!(
                "{base} Base {{ public fun read() -> Integer {{ 1 }} }} {header} module Main {{ Box.read(); let copy = Box; copy.read() }}"
            ),
            copy_type: None,
            instance: false,
            omitted: false,
        });
    }
    for (parameter, copy_type, instance, omitted) in [
        ("*items: Box", Some("Array<Box>"), false, false),
        ("**items: Box", Some("Hash<Symbol, Box>"), false, false),
        ("*items", Some("Array<Dynamic<Object>>"), false, true),
        (
            "**items",
            Some("Hash<Symbol, Dynamic<Object>>"),
            false,
            true,
        ),
        ("items: Box", Some("Box"), true, false),
        ("key items: Box", Some("Box"), true, false),
        ("*items: typeof(Box)", None, false, false),
        ("**items: typeof(Box)", None, false, false),
    ] {
        scenarios.push(Scenario {
            text: format!(
                "class Box {{ public fun read() -> Integer {{ 1 }} }} module Main {{ fun use({parameter}) -> Nil {{ let copy = items; items.read(); copy.read() }} }}"
            ),
            copy_type,
            instance,
            omitted,
        });
    }
    scenarios
}

fn request(client: &mut Client, method: &str, mut params: Value) -> Value {
    if method == "textDocument/references" {
        params["context"] = json!({"includeDeclaration":false});
    }
    client.send(&json!({"jsonrpc":"2.0","id":2,"method":method,"params":params}));
    let response = client.response();
    assert!(response.get("error").is_none(), "{response}");
    response["result"].clone()
}

fn position(character: usize) -> Value {
    json!({"textDocument":{"uri":URI},"position":{"line":0,"character":character}})
}

fn close(client: &mut Client) {
    client.send(&json!({"jsonrpc":"2.0","method":"textDocument/didClose",
        "params":{"textDocument":{"uri":URI}}}));
    assert_eq!(client.receive()["params"]["diagnostics"], json!([]));
}

#[test]
fn definition_preserves_receiver_kind_when_headers_or_rest_annotations_are_present() {
    let mut given = Client::spawn();
    given.initialize();
    for scenario in scenarios() {
        given.open(URI, &scenario.text);
        for (offset, _) in scenario.text.match_indices(".read") {
            let when = request(&mut given, "textDocument/definition", position(offset + 1));
            let expected = if scenario.instance {
                let start = scenario.text.find("read()").unwrap();
                json!([{"uri":URI,"range":{"start":{"line":0,"character":start},
                    "end":{"line":0,"character":start+4}}}])
            } else {
                json!([])
            };
            assert_eq!(when, expected, "{}", scenario.text);
        }
        close(&mut given);
    }
    given.shutdown();
}

#[test]
fn references_exclude_fabricated_calls_when_headers_or_rest_annotations_are_present() {
    let mut given = Client::spawn();
    given.initialize();
    for scenario in scenarios() {
        given.open(URI, &scenario.text);
        let when = request(
            &mut given,
            "textDocument/references",
            position(scenario.text.find("read()").unwrap()),
        );
        let expected: Vec<_> = scenario
            .text
            .match_indices(".read")
            .filter(|_| scenario.instance)
            .map(|(offset, _)| {
                json!({"uri":URI,"range":{
                "start":{"line":0,"character":offset+1},
                "end":{"line":0,"character":offset+5}}})
            })
            .collect();
        assert_eq!(when, json!(expected), "{}", scenario.text);
        close(&mut given);
    }
    given.shutdown();
}

#[test]
fn completion_preserves_receiver_kind_when_headers_or_rest_annotations_are_present() {
    let mut given = Client::spawn();
    given.initialize();
    for scenario in scenarios() {
        given.open(URI, &scenario.text);
        for (offset, _) in scenario.text.match_indices(".read") {
            let when = request(&mut given, "textDocument/completion", position(offset + 3));
            let labels: Vec<_> = when["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|item| item["label"].as_str().unwrap())
                .collect();
            let expected = if scenario.instance {
                vec!["read"]
            } else {
                vec![]
            };
            assert_eq!(labels, expected, "{}", scenario.text);
        }
        close(&mut given);
    }
    given.shutdown();
}

#[test]
fn hints_distinguish_signature_from_body_when_rest_parameters_are_copied() {
    let mut given = Client::spawn();
    given.initialize();
    for scenario in scenarios() {
        given.open(URI, &scenario.text);
        let when = request(
            &mut given,
            "textDocument/inlayHint",
            json!({
            "textDocument":{"uri":URI},"range":{"start":{"line":0,"character":0},
            "end":{"line":0,"character":scenario.text.len()}}}),
        );
        let mut expected = Vec::new();
        if scenario.omitted {
            expected.push(
                json!({"position":{"line":0,"character":scenario.text.find("items").unwrap()+5},
                "label":": Dynamic<Object>","kind":1}),
            );
        }
        if let Some(label) = scenario.copy_type {
            expected.push(
                json!({"position":{"line":0,"character":scenario.text.find("copy =").unwrap()+4},
                "label":format!(": {label}"),"kind":1}),
            );
        }
        assert_eq!(when, json!(expected), "{}", scenario.text);
        close(&mut given);
    }
    given.shutdown();
}

#[test]
fn discard_parameters_preserve_real_bindings_when_methods_and_closures_are_queried() {
    let mut given = Client::spawn();
    given.initialize();
    for text in [
        "module Main { fun use(_: Integer, arg: String) -> Nil { let copy = arg; arg } }",
        "module Main { let outer = 1; let block = { |_: Integer, arg: String|; let copy = arg; arg; outer } }",
    ] {
        given.open(URI, text);
        let declaration = text.find("arg:").unwrap();
        let usage = text.rfind("arg").unwrap();
        let definition = request(&mut given, "textDocument/definition", position(usage));
        assert_eq!(definition[0]["range"]["start"]["character"], declaration);
        let references = request(&mut given, "textDocument/references", position(declaration));
        let expected: Vec<_> = text
            .match_indices("arg")
            .skip(1)
            .map(|(offset, _)| {
                json!({"uri":URI,"range":{
                "start":{"line":0,"character":offset},"end":{"line":0,"character":offset+3}}})
            })
            .collect();
        assert_eq!(references, json!(expected));
        let completion = request(&mut given, "textDocument/completion", position(usage + 3));
        assert_eq!(completion["items"][0]["label"], "arg");
        assert_eq!(completion["items"][0]["detail"], "String");
        let hints = request(
            &mut given,
            "textDocument/inlayHint",
            json!({
            "textDocument":{"uri":URI},"range":{"start":{"line":0,"character":0},
            "end":{"line":0,"character":text.len()}}}),
        );
        let mut expected = Vec::new();
        if let Some(offset) = text.find("outer =") {
            expected.push(
                json!({"position":{"line":0,"character":offset+5},"label":": Integer","kind":1}),
            );
        }
        expected.push(
            json!({"position":{"line":0,"character":text.find("copy =").unwrap()+4},
            "label":": String","kind":1}),
        );
        assert_eq!(hints, json!(expected));
        close(&mut given);
    }
    given.shutdown();
}
