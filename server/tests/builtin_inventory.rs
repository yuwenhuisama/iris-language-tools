mod support;

use serde_json::{Value, json};
use std::fs;
use support::Client;

struct Workspace {
    client: Client,
    source: String,
    missing: String,
    _directory: tempfile::TempDir,
}

impl Workspace {
    fn new(text: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        fs::write(root.join("iris.toml"),
            "manifest_version = 1\npackage_id = \"org.example.inventory\"\napi_major = 1\nversion = \"1.0.0\"\niris_major = 1\nsources = [\"main.ir\", \"missing.ir\"]\nentry_modules = []\n[permissions]\nrequired = []\noptional = []\n"
        ).unwrap();
        fs::write(root.join("main.ir"), text).unwrap();
        let source = url::Url::from_file_path(root.join("main.ir"))
            .unwrap()
            .to_string();
        let missing = url::Url::from_file_path(root.join("missing.ir"))
            .unwrap()
            .to_string();
        let mut client = Client::spawn();
        client.initialize();
        client.send(
            &json!({"jsonrpc":"2.0","method":"workspace/didChangeWorkspaceFolders",
            "params":{"event":{"removed":[],"added":[{
                "uri":url::Url::from_file_path(&root).unwrap().as_str(),"name":"inventory"
            }]}}}),
        );
        Self {
            client,
            source,
            missing,
            _directory: directory,
        }
    }

    fn query(&mut self, method: &str, character: usize) -> Value {
        self.client
            .send(&json!({"jsonrpc":"2.0","id":2,"method":method,"params":{
                "textDocument":{"uri":self.source},"position":{"line":0,"character":character}
            }}));
        let response = self.client.response();
        assert!(response.get("error").is_none(), "{response}");
        response["result"].clone()
    }

    fn recover(&mut self) {
        assert_eq!(
            self.client.open(&self.missing, "module Recovered {} ")["method"],
            "textDocument/publishDiagnostics"
        );
    }
}

#[test]
fn refuses_builtin_completion_when_manifest_source_is_missing_then_recovers_with_overlay() {
    for (receiver, expected) in [("'abc'", "replace"), ("JSON", "encode")] {
        let text = format!("module Main {{ {receiver}.");
        let mut given = Workspace::new(&text);

        let when = given.query("textDocument/completion", text.len());

        assert_eq!(when["isIncomplete"], true, "{when}");
        assert_eq!(when["items"], json!([]), "{when}");
        given.recover();
        let recovered = given.query("textDocument/completion", text.len());
        assert!(
            recovered["items"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["label"] == expected),
            "{recovered}"
        );
        given.client.shutdown();
    }
}

#[test]
fn refuses_builtin_hover_when_manifest_source_is_missing_then_recovers_with_overlay() {
    for (call, selector) in [
        ("'abc'.replace('a', 'b')", "replace"),
        ("JSON.encode(1)", "encode"),
    ] {
        let text = format!("module Main {{ {call} }}");
        let mut given = Workspace::new(&text);
        let position = text.find(selector).unwrap();

        let when = given.query("textDocument/hover", position);

        assert!(when.is_null(), "{when}");
        given.recover();
        let recovered = given.query("textDocument/hover", position);
        assert!(
            recovered["contents"]["value"]
                .as_str()
                .unwrap()
                .contains(selector),
            "{recovered}"
        );
        given.client.shutdown();
    }
}

#[test]
fn refuses_builtin_signature_when_manifest_source_is_missing_then_recovers_with_overlay() {
    for (call, selector) in [
        ("'abc'.replace('a', 'b')", "replace"),
        ("JSON.encode(1)", "encode"),
    ] {
        let text = format!("module Main {{ {call} }}");
        let mut given = Workspace::new(&text);
        let position = text.find('(').unwrap() + 1;

        let when = given.query("textDocument/signatureHelp", position);

        assert!(when.is_null(), "{when}");
        given.recover();
        let recovered = given.query("textDocument/signatureHelp", position);
        assert!(
            recovered["signatures"][0]["label"]
                .as_str()
                .unwrap()
                .contains(selector),
            "{recovered}"
        );
        given.client.shutdown();
    }
}

#[test]
fn preserves_source_assistance_when_manifest_source_is_missing() {
    let text = "class Local { public fun source_only(value: Integer) -> Bool {} } module Main { let item = Local.new(); item.source_only(1) }";
    let mut given = Workspace::new(text);
    let selector = text.rfind("source_only").unwrap();

    let when = given.query("textDocument/completion", selector);

    assert_eq!(when["isIncomplete"], true, "{when}");
    assert_eq!(when["items"].as_array().unwrap().len(), 1, "{when}");
    assert_eq!(when["items"][0]["label"], "source_only");
    let hover = given.query("textDocument/hover", selector);
    assert!(
        hover["contents"]["value"]
            .as_str()
            .unwrap()
            .contains("source_only"),
        "{hover}"
    );
    let signature = given.query("textDocument/signatureHelp", text.rfind('1').unwrap());
    assert!(
        signature["signatures"][0]["label"]
            .as_str()
            .unwrap()
            .contains("value: Integer"),
        "{signature}"
    );
    given.client.shutdown();
}
