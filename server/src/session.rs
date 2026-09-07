use std::process::ExitCode;

use lsp_server::{Connection, Message, Request, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams, InitializeParams,
    InitializeResult, PositionEncodingKind, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
};
use serde_json::json;

use crate::{documents::Documents, formatting, notifications};

enum Phase {
    Initialize,
    Initialized,
    Running,
    Shutdown,
}

pub fn run(connection: &Connection) -> anyhow::Result<ExitCode> {
    let keywords: Vec<String> = serde_json::from_str(include_str!("../../language/keywords.json"))?;
    let completions: Vec<_> = keywords
        .into_iter()
        .map(|label| CompletionItem {
            label,
            kind: Some(CompletionItemKind::KEYWORD),
            ..CompletionItem::default()
        })
        .collect();
    let mut phase = Phase::Initialize;
    let mut documents = Documents::default();
    for message in &connection.receiver {
        match message {
            Message::Request(request) => {
                let response = match phase {
                    Phase::Initialize => initialize(request, &mut phase),
                    Phase::Initialized => Response::new_err(
                        request.id,
                        -32002,
                        "awaiting initialized notification".into(),
                    ),
                    Phase::Running if request.method == "textDocument/formatting" => {
                        formatting::respond(request, &documents)
                    }
                    Phase::Running => respond(request, &completions, &mut phase),
                    Phase::Shutdown => {
                        Response::new_err(request.id, -32600, "server has shut down".into())
                    }
                };
                connection.sender.send(response.into())?;
            }
            Message::Notification(notification) => {
                if notification.method == "exit" {
                    return Ok(match phase {
                        Phase::Shutdown => ExitCode::SUCCESS,
                        Phase::Initialize | Phase::Initialized | Phase::Running => {
                            ExitCode::FAILURE
                        }
                    });
                }
                match phase {
                    Phase::Initialized if notification.method == "initialized" => {
                        match serde_json::from_value::<lsp_types::InitializedParams>(
                            notification.params,
                        ) {
                            Ok(_) => phase = Phase::Running,
                            Err(error) => notifications::log(connection, error.to_string())?,
                        }
                    }
                    Phase::Running => {
                        let method = notification.method.clone();
                        if let Err(error) =
                            notifications::handle(connection, &mut documents, notification)
                        {
                            notifications::log(
                                connection,
                                json!({"event":"notification.rejected",
                                "method":method,"error":error.to_string()})
                                .to_string(),
                            )?;
                        }
                    }
                    Phase::Initialize | Phase::Initialized | Phase::Shutdown => {}
                }
            }
            Message::Response(_) => {}
        }
    }
    Ok(ExitCode::FAILURE)
}

fn initialize(request: Request, phase: &mut Phase) -> Response {
    if request.method != "initialize" {
        return Response::new_err(request.id, -32002, "server is not initialized".into());
    }
    match serde_json::from_value::<InitializeParams>(request.params) {
        Ok(_) => {
            *phase = Phase::Initialized;
            Response::new_ok(
                request.id,
                InitializeResult {
                    capabilities: ServerCapabilities {
                        position_encoding: Some(PositionEncodingKind::UTF16),
                        text_document_sync: Some(TextDocumentSyncCapability::Options(
                            TextDocumentSyncOptions {
                                open_close: Some(true),
                                change: Some(TextDocumentSyncKind::FULL),
                                ..TextDocumentSyncOptions::default()
                            },
                        )),
                        completion_provider: Some(CompletionOptions::default()),
                        document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
                        ..ServerCapabilities::default()
                    },
                    server_info: Some(ServerInfo {
                        name: "iris-lsp".into(),
                        version: Some(env!("CARGO_PKG_VERSION").into()),
                    }),
                },
            )
        }
        Err(error) => Response::new_err(request.id, -32602, error.to_string()),
    }
}

fn respond(request: Request, completions: &[CompletionItem], phase: &mut Phase) -> Response {
    match request.method.as_str() {
        "shutdown" => match serde_json::from_value::<()>(request.params) {
            Ok(()) => {
                *phase = Phase::Shutdown;
                Response::new_ok(request.id, ())
            }
            Err(error) => Response::new_err(request.id, -32602, error.to_string()),
        },
        "textDocument/completion" => {
            match serde_json::from_value::<CompletionParams>(request.params) {
                Ok(_) => Response::new_ok(request.id, completions),
                Err(error) => Response::new_err(request.id, -32602, error.to_string()),
            }
        }
        _ => Response::new_err(request.id, -32601, "method not found".into()),
    }
}
