mod support;

use serde_json::{Value, json};
use std::{fs, path::Path};
use support::Client;

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn query(client: &mut Client, method: &str, params: &Value) -> Value {
    client.send(&json!({"jsonrpc":"2.0","id":2,"method":method,"params":params}));
    let response = client.response();
    assert!(response.get("error").is_none(), "{response}");
    response["result"].clone()
}

#[test]
fn prefers_source_signature_when_string_is_declared_locally() {
    let text = "class String { public fun replace(source_value: Integer) -> Bool { true } }\nmodule Main { let text = String.new(); text.replace(1) }";
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:shadow", text);
    let line = text.lines().nth(1).unwrap();

    let when = query(
        &mut given,
        "textDocument/signatureHelp",
        &json!({
            "textDocument":{"uri":"untitled:shadow"},
            "position":{"line":1,"character":line.rfind('1').unwrap()}
        }),
    );

    let signature = &when["signatures"][0];
    let label = signature["label"].as_str().unwrap();
    assert!(label.contains("source_value: Integer"), "{when}");
    assert!(label.ends_with(" -> Bool"), "{when}");
    assert_eq!(signature["parameters"].as_array().unwrap().len(), 1);
    given.shutdown();
}

#[test]
fn excludes_builtin_members_when_source_string_is_declared_locally() {
    let text = "class String { public fun source_only() {} }\nmodule Main { let text = String.new(); text. }";
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:shadow", text);

    let when = query(
        &mut given,
        "textDocument/completion",
        &json!({
            "textDocument":{"uri":"untitled:shadow"},
            "position":{"line":1,"character":text.lines().nth(1).unwrap().rfind('.').unwrap()+1}
        }),
    );

    let labels: Vec<_> = when["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["label"].as_str().unwrap())
        .collect();
    assert_eq!(labels, ["source_only"], "{when}");
    given.shutdown();
}

#[test]
fn refreshes_source_shadow_when_same_package_overlay_changes() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    fs::write(root.join("iris.toml"),
        "manifest_version = 1\npackage_id = \"org.example.builtins\"\napi_major = 1\nversion = \"1.0.0\"\niris_major = 1\nsources = [\"main.ir\", \"types.iris\"]\nentry_modules = []\n[permissions]\nrequired = []\noptional = []\n"
    ).unwrap();
    let caller = "module Main { let text: String = 'abc'; text.replace(1) }";
    let original = "class String { public fun replace(disk_value: Integer) -> Integer {} }";
    fs::write(root.join("main.ir"), caller).unwrap();
    fs::write(root.join("types.iris"), original).unwrap();
    let source = uri(&root.join("main.ir"));
    let target = uri(&root.join("types.iris"));
    let mut given = Client::spawn();
    given.initialize();
    given.send(
        &json!({"jsonrpc":"2.0","method":"workspace/didChangeWorkspaceFolders",
        "params":{"event":{"removed":[],"added":[{"uri":uri(&root),"name":"builtins"}]}}}),
    );
    given.open(&target, original);
    let params = json!({"textDocument":{"uri":source},
        "position":{"line":0,"character":caller.rfind('1').unwrap()}});
    let before = query(&mut given, "textDocument/signatureHelp", &params);
    assert!(
        before["signatures"][0]["label"]
            .as_str()
            .unwrap()
            .contains("disk_value: Integer"),
        "{before}"
    );
    given.send(
        &json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
        "textDocument":{"uri":target,"version":2},"contentChanges":[{"text":
        "class String { public fun replace(overlay_value: Bool) -> Bool {} }"}]}}),
    );
    assert_eq!(given.receive()["method"], "textDocument/publishDiagnostics");

    let when = query(&mut given, "textDocument/signatureHelp", &params);

    let signature = &when["signatures"][0];
    let label = signature["label"].as_str().unwrap();
    assert!(label.contains("overlay_value: Bool"), "{when}");
    assert!(label.ends_with(" -> Bool"), "{when}");
    assert_eq!(signature["parameters"].as_array().unwrap().len(), 1);
    given.shutdown();
}
