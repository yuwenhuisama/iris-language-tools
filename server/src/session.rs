use std::process::ExitCode;
use std::time::Duration;

use lsp_server::{Connection, Message, Request, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, InitializeParams, InitializeResult,
    PositionEncodingKind, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions,
};

use crate::{
    documents::Documents,
    formatting::Formatting,
    notifications,
    semantics::Semantics,
    worker::{Budget, Program},
};

enum Phase {
    Initialize,
    Initialized,
    Running,
    Shutdown,
}

pub fn run(connection: &Connection) -> anyhow::Result<ExitCode> {
    run_with_worker(connection, Program::current()?, Budget::default())
}

pub fn run_with_worker(
    connection: &Connection,
    program: Program,
    budget: Budget,
) -> anyhow::Result<ExitCode> {
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
    let mut formatting = Formatting::new(program, budget);
    let mut semantics = Semantics::new(completions)?;
    loop {
        let message = match connection.receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(message) => message,
            Err(error) if error.is_timeout() => {
                formatting.poll(connection, &documents)?;
                semantics.poll(connection, &documents)?;
                continue;
            }
            Err(_) => break,
        };
        match message {
            Message::Request(request) => {
                let response = match phase {
                    Phase::Initialize => initialize(request, &mut phase, &mut semantics),
                    Phase::Initialized => Response::new_err(
                        request.id,
                        -32002,
                        "awaiting initialized notification".into(),
                    ),
                    Phase::Running if request.method == "textDocument/formatting" => {
                        formatting.request(request, (connection, &documents))?;
                        formatting.poll(connection, &documents)?;
                        semantics.poll(connection, &documents)?;
                        continue;
                    }
                    Phase::Running
                        if matches!(
                            request.method.as_str(),
                            "textDocument/definition"
                                | "textDocument/references"
                                | "textDocument/completion"
                                | "textDocument/inlayHint"
                        ) =>
                    {
                        semantics.request(request, (connection, &documents))?;
                        formatting.poll(connection, &documents)?;
                        semantics.poll(connection, &documents)?;
                        continue;
                    }
                    Phase::Running => respond(request, &mut phase),
                    Phase::Shutdown => {
                        Response::new_err(request.id, -32600, "server has shut down".into())
                    }
                };
                if matches!(phase, Phase::Shutdown) {
                    formatting.stop(connection)?;
                    semantics.stop(connection)?;
                }
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
                        notifications::dispatch(
                            connection,
                            notification,
                            (&mut documents, &mut formatting, &mut semantics),
                        )?;
                    }
                    Phase::Initialize | Phase::Initialized | Phase::Shutdown => {}
                }
            }
            Message::Response(_) => {}
        }
        formatting.poll(connection, &documents)?;
        semantics.poll(connection, &documents)?;
    }
    Ok(ExitCode::FAILURE)
}

fn initialize(request: Request, phase: &mut Phase, semantics: &mut Semantics) -> Response {
    if request.method != "initialize" {
        return Response::new_err(request.id, -32002, "server is not initialized".into());
    }
    match serde_json::from_value::<InitializeParams>(request.params.clone()) {
        Ok(params) => {
            semantics.roots = params.workspace_folders.map_or_else(
                || {
                    request
                        .params
                        .get("rootUri")
                        .and_then(|uri| serde_json::from_value::<lsp_types::Uri>(uri.clone()).ok())
                        .into_iter()
                        .collect()
                },
                |folders| folders.into_iter().map(|folder| folder.uri).collect(),
            );
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
                        completion_provider: Some(CompletionOptions {
                            trigger_characters: Some(vec![".".into(), ":".into()]),
                            ..CompletionOptions::default()
                        }),
                        definition_provider: Some(lsp_types::OneOf::Left(true)),
                        references_provider: Some(lsp_types::OneOf::Left(true)),
                        inlay_hint_provider: Some(lsp_types::OneOf::Left(true)),
                        workspace: Some(lsp_types::WorkspaceServerCapabilities {
                            workspace_folders: Some(
                                lsp_types::WorkspaceFoldersServerCapabilities {
                                    supported: Some(true),
                                    change_notifications: Some(lsp_types::OneOf::Left(true)),
                                },
                            ),
                            ..lsp_types::WorkspaceServerCapabilities::default()
                        }),
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

fn respond(request: Request, phase: &mut Phase) -> Response {
    match request.method.as_str() {
        "shutdown" => match serde_json::from_value::<()>(request.params) {
            Ok(()) => {
                *phase = Phase::Shutdown;
                Response::new_ok(request.id, ())
            }
            Err(error) => Response::new_err(request.id, -32602, error.to_string()),
        },
        _ => Response::new_err(request.id, -32601, "method not found".into()),
    }
}
