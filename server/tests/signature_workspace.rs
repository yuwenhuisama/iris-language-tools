mod support;

use serde_json::{Value, json};
use std::{fs, path::Path};
use support::Client;

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn signature(client: &mut Client, source: &str) -> Value {
    client.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/signatureHelp",
        "params":{"textDocument":{"uri":source},"position":{"line":0,"character":46}}}),
    );
    client.response()
}

#[test]
fn refreshes_signature_and_docs_when_same_package_target_overlay_changes() {
    for (source_name, target_name) in [("main.ir", "types.iris"), ("main.iris", "types.ir")] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        fs::write(root.join("iris.toml"), format!(
            "manifest_version = 1\npackage_id = \"org.example.signature\"\napi_major = 1\nversion = \"1.0.0\"\niris_major = 1\nsources = [\"{source_name}\", \"{target_name}\"]\nentry_modules = []\n[permissions]\nrequired = []\noptional = []\n"
        )).unwrap();
        let caller = "module Main { let item = Box.new(); item.read(1) }";
        let original =
            "class Box {\n/// Original docs\npublic fun read(value: Integer) -> Integer {}\n}";
        fs::write(root.join(source_name), caller).unwrap();
        fs::write(root.join(target_name), original).unwrap();
        let mut given = Client::spawn();
        given.initialize();
        given.send(
            &json!({"jsonrpc":"2.0","method":"workspace/didChangeWorkspaceFolders","params":{
            "event":{"removed":[],"added":[{"uri":uri(&root),"name":"signature"}]}}}),
        );
        let target = uri(&root.join(target_name));
        let source = uri(&root.join(source_name));
        given.open(&target, original);
        let before = signature(&mut given, &source);
        assert_eq!(
            before["result"]["signatures"][0]["documentation"]["value"], "Original docs",
            "{before}"
        );
        given.send(&json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{
            "textDocument":{"uri":target,"version":2},"contentChanges":[{"text":
            "class Box {\n/// Updated <docs>\npublic fun read(replacement: String) -> String {}\n}"}]}}));
        assert_eq!(given.receive()["method"], "textDocument/publishDiagnostics");

        let when = signature(&mut given, &source);

        assert_eq!(
            when["result"]["signatures"][0]["label"],
            "public fun read(replacement: String) -> String"
        );
        assert_eq!(
            when["result"]["signatures"][0]["documentation"]["value"],
            "Updated <docs>"
        );
        assert_eq!(when["result"]["activeParameter"], 0);
        assert_eq!(
            fs::read_to_string(root.join(target_name)).unwrap(),
            original
        );
        assert_eq!(fs::read_to_string(root.join(source_name)).unwrap(), caller);
        given.shutdown();
    }
}
