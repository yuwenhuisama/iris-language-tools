mod support;

use serde_json::{Value, json};
use support::Client;

fn initialized(formats: &[&str]) -> Client {
    let mut client = Client::spawn();
    client.send(
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
        "capabilities":{"textDocument":{"hover":{"contentFormat":formats}}}}}),
    );
    assert_eq!(
        client.receive()["result"]["capabilities"]["hoverProvider"],
        true
    );
    client.send(&json!({"jsonrpc":"2.0","method":"initialized","params":{}}));
    client
}

fn hover(client: &mut Client) -> Value {
    client.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{
        "textDocument":{"uri":"untitled:hover"},"position":{"line":0,"character":19}}}),
    );
    client.response()["result"]["contents"].clone()
}

#[test]
fn fences_markdown_when_signature_contains_default_string_markup() {
    let mut given = initialized(&["markdown", "plaintext"]);
    given.open(
        "untitled:hover",
        "module Main { fun read(value = '[go](command:run) <b>& ```') {} }",
    );

    let when = hover(&mut given);

    assert_eq!(
        when,
        json!({"kind":"markdown","value":
        "````iris\nprivate fun read(value: Dynamic<Object> = '[go](command:run) <b>& ```') -> Dynamic<Object>\n````\n\n**Kind:** Method\n\n**Owner:** Main\n\n**Declared return:** Dynamic&lt;Object&gt;"})
    );
    given.shutdown();
}

#[test]
fn preserves_default_string_when_plaintext_is_preferred() {
    let mut given = initialized(&["plaintext", "markdown"]);
    given.open(
        "untitled:hover",
        "module Main { fun read(value = 'a  b `[go](command:run)` <b>') {} }",
    );

    let when = hover(&mut given);

    assert_eq!(
        when,
        json!({"kind":"plaintext","value":
        "private fun read(value: Dynamic<Object> = 'a  b `[go](command:run)` <b>') -> Dynamic<Object>\n\nKind: Method\n\nOwner: Main\n\nDeclared return: Dynamic<Object>"})
    );
    given.shutdown();
}

#[test]
fn uses_plaintext_when_format_preference_is_empty() {
    let mut given = initialized(&[]);
    given.open("untitled:hover", "module Main { fun read() {} }");

    let when = hover(&mut given);

    assert_eq!(
        when,
        json!({"kind":"plaintext","value":"private fun read() -> Dynamic<Object>\n\nKind: Method\n\nOwner: Main\n\nDeclared return: Dynamic<Object>"})
    );
    given.shutdown();
}

#[test]
fn bounds_balanced_content_when_default_has_large_backtick_run() {
    let mut given = initialized(&["markdown"]);
    given.open(
        "untitled:hover",
        &format!(
            "module Main {{ fun read(value = '{}') {{}} }}",
            "`".repeat(12_000)
        ),
    );

    let when = hover(&mut given);

    let value = when["value"].as_str().unwrap();
    assert_eq!(when["kind"], "markdown");
    assert!(value.len() <= 8192);
    let lines: Vec<_> = value.lines().collect();
    let fence = lines[0].strip_suffix("iris").unwrap();
    assert_eq!(lines[2], fence);
    assert!(lines[1].ends_with("..."));
    assert!(
        fence.len()
            > lines[1]
                .split(|scalar| scalar != '`')
                .map(str::len)
                .max()
                .unwrap()
    );
    given.shutdown();
}

#[test]
fn defaults_to_plaintext_when_method_hover_capability_is_missing() {
    let mut given = Client::spawn();
    given.initialize();
    given.open("untitled:hover", "module Main { fun read() {} }");

    let when = hover(&mut given);

    assert_eq!(when["kind"], "plaintext");
    given.shutdown();
}

#[test]
fn renders_literal_docs_when_either_comment_form_is_attached() {
    for comment in [
        "/// First paragraph.\n///\n/// ![image](https://example.test/a) <b> [run](command:run)",
        "/**\n * First paragraph.\n *\n * ![image](https://example.test/a) <b> [run](command:run)\n */",
    ] {
        let mut given = initialized(&["markdown"]);
        let text =
            format!("module Main {{\n{comment}\npublic fun read() -> String {{}}\nread() }}");
        given.open("untitled:hover", &text);

        given.send(
            &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{
            "textDocument":{"uri":"untitled:hover"},
            "position":{"line":text.lines().count() - 1,"character":1}}}),
        );
        let when = given.response()["result"]["contents"].clone();

        assert_eq!(when["kind"], "markdown");
        let value = when["value"].as_str().unwrap();
        assert!(value.starts_with("```iris\npublic fun read() -> String\n```"));
        assert!(value.contains("**Owner:** Main"));
        assert!(value.contains("**Declared return:** String"));
        assert!(value.contains("**Documentation**\n\nFirst paragraph\\.\n\n\\!\\[image\\]"));
        assert!(value.contains("&lt;b&gt; \\[run\\]\\(command\\:run\\)"));
        assert!(!value.contains("![image]("));
        given.shutdown();
    }
}

#[test]
fn distinguishes_body_type_when_hovering_rest_parameter() {
    let mut given = initialized(&["plaintext"]);
    given.open(
        "untitled:hover",
        "module Main { fun read(*items: Integer) { items } }",
    );

    given.send(
        &json!({"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{
        "textDocument":{"uri":"untitled:hover"},"position":{"line":0,"character":42}}}),
    );
    let when = given.response()["result"]["contents"].clone();

    assert_eq!(
        when,
        json!({"kind":"plaintext","value":
        "*items: Integer\n\nKind: Parameter\n\nOwner: Main::read\n\nType: Array<Integer>\n\nParameter: Rest"})
    );
    given.shutdown();
}
