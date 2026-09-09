mod support;

use serde_json::{Value, json};
use support::Client;

fn query(method: &str, text: &str) -> Value {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:receiver-values", text);

    given.send(&json!({"jsonrpc":"2.0","id":2,"method":method,"params":{
        "textDocument":{"uri":"untitled:receiver-values"},
        "position":{"line":0,"character":text.len()}
    }}));
    let when = given.response();

    assert!(when.get("error").is_none(), "{text}: {when}");
    given.shutdown();
    when["result"].clone()
}

#[test]
fn completes_metadata_when_receiver_values_are_typed_or_copied() {
    for (text, selector, kind) in [
        (
            "module Main { fun use(value: Class) { value.",
            "define_method",
            2,
        ),
        (
            "module Main { fun use(value: Class) { let copy = value; copy.",
            "same?",
            2,
        ),
        (
            "class Box {} module Main { let value = Box; let copy = value; copy.",
            "define_method",
            2,
        ),
        (
            "module Main { let value = Float64; let copy = value; copy.",
            "from_bits",
            2,
        ),
        (
            "contract C {} module Main { let value = C; value.",
            "parents",
            2,
        ),
        ("module M {} module Main { M.", "modules", 10),
    ] {
        let given = text;

        let when = query("textDocument/completion", given);

        assert!(
            when["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["label"] == selector && item["kind"] == kind),
            "{text}: {when}"
        );
    }
}

#[test]
fn returns_signatures_when_receiver_is_a_class_or_contract_value() {
    for (text, arity) in [
        (
            "module Main { fun use(value: Class) { value.define_method(",
            2,
        ),
        (
            "class Box {} module Main { let value = Box; value.define_method(",
            2,
        ),
        ("module Main { let value = Float64; value.from_bits(", 1),
        ("contract C {} module Main { C.parents(", 0),
        (
            "contract C {} module Main { let value = C; value.parents(",
            0,
        ),
    ] {
        let given = text;

        let when = query("textDocument/signatureHelp", given);

        assert_eq!(
            when["signatures"][0]["parameters"]
                .as_array()
                .unwrap_or_else(|| panic!("{text}: {when}"))
                .len(),
            arity
        );
    }
}

#[test]
fn refuses_signatures_when_method_reads_are_not_calls() {
    for given in [
        "module Main { Object.new().hash.div(",
        "module Main { Object.new().to_string.replace(",
        "module Main { let value = Object.new(); let method = value.hash; method.div(",
    ] {
        let when = query("textDocument/signatureHelp", given);

        assert_eq!(when, Value::Null, "{given}");
    }
}

#[test]
fn returns_signatures_when_methods_are_called_or_properties_are_read() {
    for given in [
        "module Main { Object.new().hash().div(",
        "module Main { Object.new().to_string().replace(",
        "module Main { let value = Float64; value.nan.to_bits(",
        "module Main { fun use(value: Class) { value.modules.push(",
        "contract C {} module Main { C.parents.push(",
        "module M {} module Main { M.modules.push(",
    ] {
        let when = query("textDocument/signatureHelp", given);

        assert!(
            when["signatures"]
                .as_array()
                .is_some_and(|signatures| !signatures.is_empty()),
            "{given}: {when}"
        );
    }
}

#[test]
fn refuses_signatures_when_receiver_evidence_does_not_establish_the_route() {
    for given in [
        "module Main { let value = JSON; value.encode(",
        "module Main { let value = Encoding::UTF_8; value.decode(",
        "module Main { mut value = Float64; value.from_bits(",
        "contract C {} module Main { fun use(value: C) { value.parents(",
        "class Box {} class Box {} module Main { let value = Box; value.define_method(",
        "class Box {} open class Box {} module Main { let value = Box; value.define_method(",
        "class Box { private class fun define_method(own) {} } module Main { let value = Box; value.define_method(",
        "module Main { let Float64 = 1; let value = Float64; value.from_bits(",
        "module M {} module Main { M.modules(",
    ] {
        let when = query("textDocument/signatureHelp", given);

        assert_eq!(when, Value::Null, "{given}");
    }
}
