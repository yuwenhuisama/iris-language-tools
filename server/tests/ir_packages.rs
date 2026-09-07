mod support;

use serde_json::{Value, json};
use std::fs;
use support::Client;

fn query(client: &mut Client, method: &str, params: &Value) -> Value {
    client.send(&json!({"jsonrpc":"2.0","id":2,"method":method,"params":params}));
    client.response()
}

#[test]
fn resolves_mixed_suffix_packages_when_disk_watched_and_overlay_targets_change() {
    for (main, types) in [("main.ir", "types.iris"), ("main.iris", "types.ir")] {
        let given = tempfile::tempdir().unwrap();
        let root = given.path().canonicalize().unwrap();
        let uri = |name: &str| {
            url::Url::from_file_path(root.join(name))
                .unwrap()
                .to_string()
        };
        let source = "module Main {\nlet item = Box.new()\nitem.re\n}";
        fs::write(root.join(main), source).unwrap();
        fs::write(root.join(types), "class Box { public fun read() {} }").unwrap();
        fs::write(root.join("iris.toml"), format!(
            "manifest_version = 1\npackage_id = \"org.example.mixed\"\napi_major = 1\nversion = \"1.0.0\"\niris_major = 1\nsources = {main_types:?}\nentry_modules = []\n[permissions]\nrequired = []\noptional = []\n",
            main_types = [main, types]
        )).unwrap();
        let mut client = Client::spawn();
        client.initialize();
        client.send(
            &json!({"jsonrpc":"2.0","method":"workspace/didChangeWorkspaceFolders","params":{
            "event":{"added":[{"uri":uri(""),"name":"mixed"}],"removed":[]}}}),
        );

        for (line, member) in [(0, "read"), (1, "readDisk"), (2, "readOverlay")] {
            let target = format!(
                "{}class Box {{ public fun {member}() {{}} }}",
                "\n".repeat(line)
            );
            match line {
                0 => {}
                1 => {
                    fs::write(root.join(types), &target).unwrap();
                    client.send(&json!({"jsonrpc":"2.0","method":"workspace/didChangeWatchedFiles","params":{
                        "changes":[{"uri":uri(types),"type":2}]}}));
                }
                2 => {
                    client.open(&uri(types), &target);
                }
                _ => unreachable!(),
            }
            let position =
                json!({"textDocument":{"uri":uri(main)},"position":{"line":1,"character":12}});
            let when = query(&mut client, "textDocument/definition", &position);
            let declaration = json!({"uri":uri(types),"range":{
                "start":{"line":line,"character":6},"end":{"line":line,"character":9}}});
            assert_eq!(when["result"], json!([declaration]));

            let mut params = position;
            params["context"] = json!({"includeDeclaration":true});
            let when = query(&mut client, "textDocument/references", &params);
            let locations = when["result"].as_array().unwrap();
            assert_eq!(locations.len(), 2);
            assert!(locations.contains(&declaration));
            assert!(locations.contains(&json!({"uri":uri(main),"range":{
                "start":{"line":1,"character":11},"end":{"line":1,"character":14}}})));

            let when = query(
                &mut client,
                "textDocument/completion",
                &json!({
                "textDocument":{"uri":uri(main)},"position":{"line":2,"character":7}}),
            );
            assert_eq!(when["result"]["isIncomplete"], false);
            let items = when["result"]["items"].as_array().unwrap();
            assert_eq!(items.len(), 1);
            assert_eq!(items[0]["label"], member);
            assert_eq!(
                items[0]["textEdit"],
                json!({"newText":member,"range":{
                "start":{"line":2,"character":5},"end":{"line":2,"character":7}}})
            );
        }

        client.open(&uri(main), source);
        let when = query(
            &mut client,
            "textDocument/formatting",
            &json!({
            "textDocument":{"uri":uri(main)},"options":{"tabSize":2,"insertSpaces":true}}),
        );
        assert_eq!(
            when["result"],
            json!([{"range":{
            "start":{"line":0,"character":0},"end":{"line":3,"character":1}},
            "newText":"module Main {\n  let item = Box.new()\n  item.re\n}\n"}])
        );
        client.shutdown();
    }
}
