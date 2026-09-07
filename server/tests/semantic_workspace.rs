mod support;

use serde_json::{Value, json};
use std::{fs, path::Path};
use support::Client;

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn initialize(client: &mut Client, root: &Path) {
    client.send(
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "capabilities":{},"workspaceFolders":[{"uri":uri(root),"name":"test"}]}}),
    );
    let capabilities = client.receive()["result"]["capabilities"].clone();
    assert_eq!(capabilities["definitionProvider"], true);
    assert_eq!(capabilities["referencesProvider"], true);
    assert_eq!(capabilities["inlayHintProvider"], true);
    assert_eq!(
        capabilities["completionProvider"]["triggerCharacters"],
        json!([".", ":"])
    );
    client.send(&json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
}

fn manifest(root: &Path, sources: &[&str]) {
    fs::write(root.join("iris.toml"), format!(
        "manifest_version = 1\npackage_id = \"org.example.semantic\"\napi_major = 1\nversion = \"1.0.0\"\niris_major = 1\nsources = {sources:?}\nentry_modules = []\n[permissions]\nrequired = []\noptional = []\n"
    )).unwrap();
}

fn definition(client: &mut Client, source: &str) -> Value {
    client.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{
        "textDocument":{"uri":source},"position":{"line":0,"character":25}}}),
    );
    client.response()
}

#[test]
fn resolves_cross_file_target_when_package_and_unsaved_overlay_share_identity() {
    let given = tempfile::tempdir().unwrap();
    let root = given.path().canonicalize().unwrap();
    manifest(&root, &["main.iris", "types.iris"]);
    fs::write(
        root.join("main.iris"),
        "module Main { let item = Box.new() }",
    )
    .unwrap();
    fs::write(root.join("types.iris"), "class Old {}").unwrap();
    let mut client = Client::spawn();
    initialize(&mut client, &root);
    let target = uri(&root.join("types.iris"));
    client.open(&target, "// \u{1f600}\r\nclass Box {}");
    let when = definition(&mut client, &uri(&root.join("main.iris")));
    assert_eq!(
        when["result"],
        json!([{"uri":target,"range":{
        "start":{"line":1,"character":6},"end":{"line":1,"character":9}}}])
    );
    client.shutdown();
}

#[test]
fn refreshes_disk_target_when_watched_source_changes() {
    let given = tempfile::tempdir().unwrap();
    let root = given.path().canonicalize().unwrap();
    manifest(&root, &["main.iris", "types.iris"]);
    fs::write(
        root.join("main.iris"),
        "module Main { let item = Box.new() }",
    )
    .unwrap();
    fs::write(root.join("types.iris"), "class Box {}").unwrap();
    let mut client = Client::spawn();
    initialize(&mut client, &root);
    let source = uri(&root.join("main.iris"));
    assert_eq!(
        definition(&mut client, &source)["result"][0]["range"]["start"]["line"],
        0
    );
    fs::write(root.join("types.iris"), "\nclass Box {}").unwrap();
    client.send(
        &json!({"jsonrpc":"2.0","method":"workspace/didChangeWatchedFiles","params":{
        "changes":[{"uri":uri(&root.join("types.iris")),"type":2}]}}),
    );
    let when = definition(&mut client, &source);
    assert_eq!(when["result"][0]["range"]["start"]["line"], 1);
    client.shutdown();
}

#[test]
fn rejects_exhaustive_references_when_manifest_inventory_is_incomplete() {
    let given = tempfile::tempdir().unwrap();
    let root = given.path().canonicalize().unwrap();
    manifest(&root, &["main.iris", "missing.iris"]);
    fs::write(
        root.join("main.iris"),
        "module Main { let local = 1; local }",
    )
    .unwrap();
    let mut client = Client::spawn();
    initialize(&mut client, &root);
    client.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{
        "textDocument":{"uri":uri(&root.join("main.iris"))},"position":{"line":0,"character":30},
        "context":{"includeDeclaration":true}}}),
    );
    let when = client.response();
    assert_eq!(when["error"]["code"], -32803);
    client.shutdown();
}

#[test]
fn removes_package_sources_when_workspace_folder_is_removed() {
    let given = tempfile::tempdir().unwrap();
    let root = given.path().canonicalize().unwrap();
    manifest(&root, &["main.iris", "types.iris"]);
    fs::write(
        root.join("main.iris"),
        "module Main { let item = Box.new() }",
    )
    .unwrap();
    fs::write(root.join("types.iris"), "class Box {}").unwrap();
    let mut client = Client::spawn();
    initialize(&mut client, &root);
    let source = uri(&root.join("main.iris"));
    assert_eq!(
        definition(&mut client, &source)["result"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    client.send(
        &json!({"jsonrpc":"2.0","method":"workspace/didChangeWorkspaceFolders","params":{
        "event":{"added":[],"removed":[{"uri":uri(&root),"name":"test"}]}}}),
    );
    let when = definition(&mut client, &source);
    assert_eq!(when["result"], json!([]));
    client.shutdown();
}

#[test]
fn adds_package_sources_when_workspace_folder_is_added() {
    let given = tempfile::tempdir().unwrap();
    let root = given.path().canonicalize().unwrap();
    manifest(&root, &["main.iris", "types.iris"]);
    fs::write(
        root.join("main.iris"),
        "module Main { let item = Box.new() }",
    )
    .unwrap();
    fs::write(root.join("types.iris"), "class Box {}").unwrap();
    let mut client = Client::spawn();
    client.initialize();
    let source = uri(&root.join("main.iris"));
    assert_eq!(definition(&mut client, &source)["result"], json!([]));
    client.send(
        &json!({"jsonrpc":"2.0","method":"workspace/didChangeWorkspaceFolders","params":{
        "event":{"removed":[],"added":[{"uri":uri(&root),"name":"test"}]}}}),
    );
    let when = definition(&mut client, &source);
    assert_eq!(when["result"].as_array().unwrap().len(), 1);
    client.shutdown();
}

#[test]
fn uses_root_uri_when_workspace_folders_are_absent() {
    let given = tempfile::tempdir().unwrap();
    let root = given.path().canonicalize().unwrap();
    manifest(&root, &["main.iris", "types.iris"]);
    fs::write(
        root.join("main.iris"),
        "module Main { let item = Box.new() }",
    )
    .unwrap();
    fs::write(root.join("types.iris"), "class Box {}").unwrap();
    let mut client = Client::spawn();
    client.send(
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "capabilities":{},"rootUri":uri(&root)}}),
    );
    assert!(client.receive()["result"]["capabilities"].is_object());
    client.send(&json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
    let when = definition(&mut client, &uri(&root.join("main.iris")));
    assert_eq!(when["result"].as_array().unwrap().len(), 1);
    client.shutdown();
}

#[test]
fn keeps_standalone_files_isolated_when_names_match_across_buffers() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:types", "class Box {}");
    given.open("untitled:main", "module Main { let item = Box.new() }");
    let when = definition(&mut given, "untitled:main");
    assert_eq!(when["result"], json!([]));
    given.shutdown();
}
