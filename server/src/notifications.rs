use lsp_server::{Connection, Notification};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    LogMessageParams, MessageType, PublishDiagnosticsParams, Uri,
};
use serde_json::json;

use crate::{diagnostics::analyze, documents::Documents};

pub fn dispatch(
    connection: &Connection,
    notification: Notification,
    state: (
        &mut Documents,
        &mut crate::formatting::Formatting,
        &mut crate::semantics::Semantics,
    ),
) -> anyhow::Result<()> {
    let (documents, formatting, semantics) = state;
    let method = notification.method.clone();
    let result = if method == "$/cancelRequest" {
        formatting
            .cancel(connection, notification.params.clone())
            .and_then(|()| semantics.cancel(connection, notification.params))
    } else if method.starts_with("workspace/") || method == "textDocument/didSave" {
        semantics.notification(notification)
    } else {
        handle(connection, documents, notification)
            .and_then(|changed| if changed { semantics.changed() } else { Ok(()) })
    };
    if let Err(error) = result {
        log(
            connection,
            json!({"event":"notification.rejected", "method":method,
            "error":error.to_string()})
            .to_string(),
        )?;
    }
    Ok(())
}

pub fn log(connection: &Connection, message: String) -> anyhow::Result<()> {
    connection.sender.send(
        Notification::new(
            "window/logMessage".into(),
            LogMessageParams {
                typ: MessageType::WARNING,
                message,
            },
        )
        .into(),
    )?;
    Ok(())
}

pub fn handle(
    connection: &Connection,
    documents: &mut Documents,
    notification: Notification,
) -> anyhow::Result<bool> {
    match notification.method.as_str() {
        "textDocument/didOpen" => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(notification.params)?;
            if let Some(uri) = documents.open(params.text_document) {
                publish(connection, documents, &uri)?;
                return Ok(true);
            }
        }
        "textDocument/didChange" => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(notification.params)?;
            if let Some(uri) = documents.change(params)? {
                publish(connection, documents, &uri)?;
                return Ok(true);
            }
        }
        "textDocument/didClose" => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(notification.params)?;
            let uri = params.text_document.uri;
            if documents.close(&uri).is_some() {
                connection.sender.send(
                    Notification::new(
                        "textDocument/publishDiagnostics".into(),
                        PublishDiagnosticsParams::new(uri, Vec::new(), None),
                    )
                    .into(),
                )?;
                return Ok(true);
            }
        }
        _ => {}
    }
    Ok(false)
}

fn publish(connection: &Connection, documents: &Documents, uri: &Uri) -> anyhow::Result<()> {
    if let Some(document) = documents.get(uri) {
        let (diagnostics, unlocated) = analyze(&document.text);
        connection.sender.send(
            Notification::new(
                "textDocument/publishDiagnostics".into(),
                PublishDiagnosticsParams::new(uri.clone(), diagnostics, Some(document.version)),
            )
            .into(),
        )?;
        for code in unlocated {
            log(
                connection,
                json!({"event":"lexer.unlocated", "code":code,
                "uri":uri, "version":document.version})
                .to_string(),
            )?;
        }
    }
    Ok(())
}
