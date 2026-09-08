mod support;

use serde_json::{Value, json};
use std::{fs, path::Path};
use support::Client;

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn package(root: &Path, sources: &[&str]) {
    fs::write(root.join("iris.toml"), format!(
        "manifest_version = 1\npackage_id = \"org.example.hover\"\napi_major = 1\nversion = \"1.0.0\"\niris_major = 1\nsources = {sources:?}\nentry_modules = []\n[permissions]\nrequired = []\noptional = []\n"
    )).unwrap();
}

fn client(root: &Path) -> Client {
    let mut client = Client::spawn();
    client.initialize();
    client.send(
        &json!({"jsonrpc":"2.0","method":"workspace/didChangeWorkspaceFolders","params":{
        "event":{"removed":[],"added":[{"uri":uri(root),"name":"hover"}]}}}),
    );
    client
}

fn hover(client: &mut Client, source: &str, position: (u32, u32)) -> Value {
    client.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{
        "textDocument":{"uri":source},"position":{"line":position.0,"character":position.1}}}),
    );
    client.response()
}

#[test]
fn uses_caller_utf16_range_when_target_is_in_another_file() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    package(&root, &["main.ir", "types.iris"]);
    fs::write(
        root.join("main.ir"),
        "module Main {\r\n '\u{1f600}'; Box.new()\r\n}",
    )
    .unwrap();
    fs::write(root.join("types.iris"), "\n\n\nclass Box<T> {}").unwrap();
    let mut given = client(&root);

    let when = hover(&mut given, &uri(&root.join("main.ir")), (1, 8));

    assert_eq!(
        when,
        json!({"id":2,"result":{
            "contents":{"kind":"plaintext","value":"class Box<T>\n\nKind: Class"},
            "range":{"start":{"line":1,"character":7},"end":{"line":1,"character":10}}
        }})
    );
    given.shutdown();
}

#[test]
fn updates_known_return_when_dirty_target_changes() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    package(&root, &["main.iris", "types.ir"]);
    fs::write(
        root.join("main.iris"),
        "module Main { let item = Box.new(); let value = item.read(); value }",
    )
    .unwrap();
    fs::write(
        root.join("types.ir"),
        "class Box { public fun read() -> Integer { 1 } }",
    )
    .unwrap();
    let mut given = client(&root);
    let target = uri(&root.join("types.ir"));
    given.open(&target, "class Box { public fun read() -> Integer { 1 } }");
    let source = uri(&root.join("main.iris"));
    assert_eq!(
        hover(&mut given, &source, (0, 61)),
        json!({"id":2,"result":{
            "contents":{"kind":"plaintext","value":"let value: Integer\n\nKind: Variable\n\nOwner: Main\n\nType: Integer"},
            "range":{"start":{"line":0,"character":61},"end":{"line":0,"character":66}}
        }})
    );
    given.send(
        &json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
        "textDocument":{"uri":target,"version":2},
        "contentChanges":[{"text":"class Box { public fun read() -> String { 'new' } }"}]}}),
    );
    assert_eq!(given.receive()["method"], "textDocument/publishDiagnostics");

    let when = hover(&mut given, &source, (0, 61));

    assert_eq!(
        when,
        json!({"id":2,"result":{
            "contents":{"kind":"plaintext","value":"let value: String\n\nKind: Variable\n\nOwner: Main\n\nType: String"},
            "range":{"start":{"line":0,"character":61},"end":{"line":0,"character":66}}
        }})
    );
    given.shutdown();
}

#[test]
fn preserves_inventory_error_when_requested_source_is_missing() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    package(&root, &["main.iris", "missing.iris"]);
    fs::write(
        root.join("main.iris"),
        "module Main { let value = 1; value }",
    )
    .unwrap();
    let mut given = client(&root);

    let when = hover(&mut given, &uri(&root.join("missing.iris")), (0, 0));

    assert_eq!(when["error"]["code"], -32803);
    assert!(
        when["error"]["data"]["inventory"]["totalCount"]
            .as_u64()
            .unwrap()
            > 0
    );
    given.shutdown();
}

#[test]
fn returns_local_hover_when_unrelated_inventory_is_partial() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    package(&root, &["main.iris", "missing.iris"]);
    fs::write(
        root.join("main.iris"),
        "module Main { let value = 1; value }",
    )
    .unwrap();
    let mut given = client(&root);

    let when = hover(&mut given, &uri(&root.join("main.iris")), (0, 29));

    assert_eq!(
        when,
        json!({"id":2,"result":{
            "contents":{"kind":"plaintext","value":"let value: Integer\n\nKind: Variable\n\nOwner: Main\n\nType: Integer"},
            "range":{"start":{"line":0,"character":29},"end":{"line":0,"character":34}}
        }})
    );
    given.shutdown();
}
