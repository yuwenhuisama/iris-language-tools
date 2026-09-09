mod support;

use serde_json::{Value, json};
use support::Client;

const CURSOR: &str = "<cursor>";

fn query(method: &str, marked: &str) -> Value {
    let byte = marked.find(CURSOR).unwrap();
    let prefix = &marked[..byte];
    let text = marked.replace(CURSOR, "");
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:builtins", &text);
    let mut params = json!({"textDocument":{"uri":"untitled:builtins"},
        "position":{"line":prefix.bytes().filter(|byte| *byte == b'\n').count(),
        "character":prefix.rsplit('\n').next().unwrap().encode_utf16().count()}});
    if method == "textDocument/references" {
        params["context"] = json!({"includeDeclaration":true});
    }

    given.send(&json!({"jsonrpc":"2.0","id":2,"method":method,"params":params}));
    let when = given.response();

    assert!(when.get("error").is_none(), "{marked}: {when}");
    given.shutdown();
    when["result"].clone()
}

fn labels(result: &Value) -> Vec<&str> {
    result["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["label"].as_str().unwrap())
        .collect()
}

#[test]
fn completes_string_methods_when_receiver_is_a_literal() {
    let given = "module Main { 'abc'.<cursor> }";

    let when = query("textDocument/completion", given);

    let labels = labels(&when);
    assert!(labels.contains(&"replace"), "{when}");
    assert!(labels.contains(&"trim"), "{when}");
    assert!(!labels.contains(&"push"), "{when}");
}

#[test]
fn completes_string_methods_when_receiver_is_an_inferred_binding() {
    let given = "module Main { let text = 'abc'; text.<cursor> }";

    let when = query("textDocument/completion", given);

    let labels = labels(&when);
    assert!(labels.contains(&"replace"), "{when}");
    assert!(labels.contains(&"trim"), "{when}");
    assert!(!labels.contains(&"push"), "{when}");
}

#[test]
fn returns_selector_utf16_range_when_hovering_string_replace() {
    let given = "module Main {\r\n '\u{1f600}'.re<cursor>place('a', 'b')\r\n}";

    let when = query("textDocument/hover", given);

    assert_eq!(
        when["range"],
        json!({
            "start":{"line":1,"character":6},"end":{"line":1,"character":13}
        }),
        "{when}"
    );
    let content = when["contents"]["value"].as_str().unwrap();
    assert!(content.contains("replace("), "{content}");
    assert!(content.contains("Owner: String"), "{content}");
    assert!(content.contains("Kind: Method"), "{content}");
}

#[test]
fn selects_second_parameter_when_string_replace_has_a_comma() {
    let given = "module Main { 'abc'.replace('a', <cursor>) }";

    let when = query("textDocument/signatureHelp", given);

    assert_eq!(when["activeSignature"], 0, "{when}");
    assert_eq!(when["activeParameter"], 1, "{when}");
    let signatures = when["signatures"].as_array().unwrap();
    assert_eq!(signatures.len(), 1, "{when}");
    let signature = &signatures[0];
    let label = signature["label"].as_str().unwrap();
    assert!(label.contains("replace("), "{label}");
    assert!(label.ends_with(" -> String"), "{label}");
    let parameters = signature["parameters"].as_array().unwrap();
    assert_eq!(parameters.len(), 2, "{when}");
    for parameter in parameters {
        let parameter_label = parameter["label"].as_str().unwrap();
        assert!(label.contains(parameter_label), "{when}");
        assert!(parameter_label.contains("String"), "{when}");
    }
}

#[test]
fn completes_class_methods_when_receiver_is_float64_class() {
    let given = "module Main { Float64.<cursor> }";

    let when = query("textDocument/completion", given);

    let labels = labels(&when);
    assert!(labels.contains(&"from_bits"), "{when}");
    assert!(!labels.contains(&"to_bits"), "{when}");
}

#[test]
fn completes_instance_methods_when_receiver_is_float64_literal() {
    let given = "module Main { let number = 1.0; number.<cursor> }";

    let when = query("textDocument/completion", given);

    let labels = labels(&when);
    assert!(labels.contains(&"to_bits"), "{when}");
    assert!(!labels.contains(&"from_bits"), "{when}");
}

#[test]
fn returns_class_signature_when_calling_float64_from_bits() {
    let given = "module Main { Float64.from_bits(<cursor>0) }";

    let when = query("textDocument/signatureHelp", given);

    assert_eq!(when["activeParameter"], 0, "{when}");
    let signature = &when["signatures"][0];
    assert!(signature["label"].as_str().unwrap().contains("from_bits("));
    assert_eq!(signature["parameters"].as_array().unwrap().len(), 1);
}

#[test]
fn returns_null_when_class_method_is_called_on_float64_instance() {
    let given = "module Main { let number = 1.0; number.from_bits(<cursor>0) }";

    let when = query("textDocument/signatureHelp", given);

    assert_eq!(when, Value::Null);
}

#[test]
fn completes_array_methods_when_receiver_is_a_literal() {
    let given = "module Main { [1, 2].<cursor> }";

    let when = query("textDocument/completion", given);

    let labels = labels(&when);
    assert!(labels.contains(&"length"), "{when}");
    assert!(labels.contains(&"push"), "{when}");
    assert!(!labels.contains(&"size"), "{when}");
    assert!(!labels.contains(&"replace"), "{when}");
}

#[test]
fn suppresses_builtin_hints_when_receiver_is_dynamic() {
    for annotation in ["Dynamic<Object>", "Dynamic<String>"] {
        for (method, expression) in [
            ("textDocument/completion", "text.<cursor>"),
            ("textDocument/hover", "text.re<cursor>place('a', 'b')"),
            ("textDocument/signatureHelp", "text.replace('a', <cursor>)"),
        ] {
            let given = format!("module Main {{ fun use(text: {annotation}) {{ {expression} }} }}");

            let when = query(method, &given);

            match method {
                "textDocument/completion" => assert!(labels(&when).is_empty(), "{when}"),
                _ => assert_eq!(when, Value::Null, "{given}"),
            }
        }
    }
}

#[test]
fn returns_empty_navigation_when_builtin_has_no_source_declaration() {
    for method in ["textDocument/definition", "textDocument/references"] {
        for given in [
            "module Main { 'abc'.re<cursor>place('a', 'b'); 'abc'.replace('a', 'c') }",
            "module Main { Float64.fr<cursor>om_bits(0); Float64.from_bits(1) }",
        ] {
            let when = query(method, given);

            assert_eq!(when, json!([]), "{method}: {given}");
        }
    }
}

#[test]
fn returns_null_when_unknown_inner_call_has_a_known_outer_call() {
    for given in [
        "module Main { 'abc'.replace('a', unknown(<cursor>)) }",
        "module Main { public fun outer(first, second) {} outer(1, 'abc'.unknown(<cursor>)) }",
    ] {
        let when = query("textDocument/signatureHelp", given);

        assert_eq!(when, Value::Null, "{given}");
    }
}
