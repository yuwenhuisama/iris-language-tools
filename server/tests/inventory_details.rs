mod support;

use serde_json::{Value, json};
use std::fs;
use support::Client;

#[test]
fn reports_bounded_inventory_details_when_listed_ir_sources_are_missing() {
    let given = tempfile::tempdir().unwrap();
    let root = given.path().canonicalize().unwrap();
    let uri = |name: &str| {
        url::Url::from_file_path(root.join(name))
            .unwrap()
            .to_string()
    };
    let missing: Vec<_> = (0..10).map(|index| format!("missing{index}.ir")).collect();
    let mut sources = vec!["main.ir".to_owned()];
    sources.extend(missing.clone());
    fs::write(root.join("iris.toml"), format!(
        "manifest_version = 1\npackage_id = \"org.example.details\"\napi_major = 1\nversion = \"1.0.0\"\niris_major = 1\nsources = {sources:?}\nentry_modules = []\n[permissions]\nrequired = []\noptional = []\n"
    )).unwrap();
    let source = "module Main { let private_source_marker = 1; private_source_marker }";
    fs::write(root.join("main.ir"), source).unwrap();
    let mut client = Client::spawn();
    client.send(
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "capabilities":{},"rootUri":uri("")}}),
    );
    assert!(client.receive()["result"]["capabilities"].is_object());
    client.send(&json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
    client.open(&uri("main.ir"), source);

    client.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{
        "textDocument":{"uri":uri("main.ir")},"position":{"line":0,"character":20},
        "context":{"includeDeclaration":true}}}),
    );
    let notification = client.receive();
    let when = client.receive();

    assert_eq!(when["error"]["code"], -32803);
    assert!(when.get("result").is_none());
    let details = &when["error"]["data"]["inventory"];
    assert_eq!(details["totalCount"], 10);
    let issues = details["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 8);
    for (issue, name) in issues.iter().zip(&missing) {
        assert_eq!(issue["kind"], "io");
        assert_eq!(issue["ioKind"], "NotFound");
        assert_eq!(issue["uri"], uri(name));
        assert_eq!(issue["path"], root.join(name).to_str().unwrap());
    }
    assert_eq!(notification["method"], "window/logMessage");
    let log: Value =
        serde_json::from_str(notification["params"]["message"].as_str().unwrap()).unwrap();
    assert_eq!(log["event"], "semantic.failed");
    assert_eq!(log["id"], 2);
    assert_eq!(log["inventory"], *details);
    assert!(!when.to_string().contains("private_source_marker"));
    assert!(!notification.to_string().contains("private_source_marker"));
    client.shutdown();
}

#[test]
fn marks_completion_partial_when_a_listed_ir_source_is_missing() {
    let given = tempfile::tempdir().unwrap();
    let root = given.path().canonicalize().unwrap();
    let uri = |name: &str| {
        url::Url::from_file_path(root.join(name))
            .unwrap()
            .to_string()
    };
    fs::write(root.join("iris.toml"),
        "manifest_version = 1\npackage_id = \"org.example.partial\"\napi_major = 1\nversion = \"1.0.0\"\niris_major = 1\nsources = [\"main.ir\", \"missing.ir\"]\nentry_modules = []\n[permissions]\nrequired = []\noptional = []\n"
    ).unwrap();
    let mut client = Client::spawn();
    client.initialize();
    client.send(
        &json!({"jsonrpc":"2.0","method":"workspace/didChangeWorkspaceFolders","params":{
        "event":{"added":[{"uri":uri(""),"name":"partial"}],"removed":[]}}}),
    );
    let source = "class Box { public fun read() {} } module Main { let item = Box.new(); item.re }";
    client.open(&uri("main.ir"), source);

    client.send(&json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{
        "textDocument":{"uri":uri("main.ir")},"position":{"line":0,"character":source.rfind("re }").unwrap()+2}}}));
    let when = client.response();

    assert_eq!(when["result"]["isIncomplete"], true);
    assert_eq!(when["result"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(when["result"]["items"][0]["label"], "read");
    client.shutdown();
}
