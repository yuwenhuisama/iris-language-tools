use std::sync::Arc;

use iris_lexer::ByteOffset;
use lsp_server::{Connection, Request, RequestId, Response};
use lsp_types::{DocumentFormattingParams, Position, Range, TextEdit, Uri};
use serde::Deserialize;
use serde_json::json;

use crate::{
    documents::Documents,
    notifications,
    positions::position,
    worker::{Budget, Failure, Program, Worker},
    worker_protocol::{INPUT_LIMIT, Outcome},
};

struct Pending {
    id: RequestId,
    uri: Uri,
    version: i32,
    generation: Arc<()>,
    end: Position,
    worker: Worker,
}

impl Pending {
    fn current(&self, documents: &Documents) -> bool {
        documents.get(&self.uri).is_some_and(|document| {
            document.version == self.version && Arc::ptr_eq(&document.generation, &self.generation)
        })
    }
}

pub struct Formatting {
    program: Program,
    budget: Budget,
    pending: Option<Pending>,
}

impl Formatting {
    pub const fn new(program: Program, budget: Budget) -> Self {
        Self {
            program,
            budget,
            pending: None,
        }
    }

    pub fn request(
        &mut self,
        request: Request,
        context: (&Connection, &Documents),
    ) -> anyhow::Result<()> {
        let (connection, documents) = context;
        let params = match serde_json::from_value::<DocumentFormattingParams>(request.params) {
            Ok(params) if (1..=16).contains(&params.options.tab_size) => params,
            Ok(_) => {
                connection.sender.send(
                    Response::new_err(
                        request.id,
                        -32602,
                        "tabSize must be between 1 and 16".into(),
                    )
                    .into(),
                )?;
                return Ok(());
            }
            Err(error) => {
                connection
                    .sender
                    .send(Response::new_err(request.id, -32602, error.to_string()).into())?;
                return Ok(());
            }
        };
        let Some(document) = documents.get(&params.text_document.uri) else {
            return finish(connection, request.id, None);
        };
        if self.pending.is_some() {
            return finish(connection, request.id, Some(format!("{:?}", Failure::Busy)));
        }
        if document.text.len() > INPUT_LIMIT {
            return finish(
                connection,
                request.id,
                Some(format!("{:?}", Failure::InputLimit)),
            );
        }
        let Some(end) = position(&document.text, ByteOffset(document.text.len())) else {
            return finish(
                connection,
                request.id,
                Some(format!("{:?}", Failure::InputLimit)),
            );
        };
        match Worker::spawn(self.program.command(), document.text.clone(), self.budget) {
            Ok(worker) => {
                self.pending = Some(Pending {
                    id: request.id,
                    uri: params.text_document.uri,
                    version: document.version,
                    generation: Arc::clone(&document.generation),
                    end,
                    worker,
                });
                Ok(())
            }
            Err(reason) => finish(connection, request.id, Some(format!("{reason:?}"))),
        }
    }

    pub fn poll(&mut self, connection: &Connection, documents: &Documents) -> anyhow::Result<()> {
        let Some(pending) = self.pending.as_mut() else {
            return Ok(());
        };
        let result = if pending.current(documents) {
            pending.worker.poll()
        } else {
            Some(Err(Failure::Stale))
        };
        if let Some(result) = result
            && let Some(pending) = self.pending.take()
        {
            let Pending {
                id, end, worker, ..
            } = pending;
            drop(worker);
            match result {
                Ok(Outcome::Changed(new_text)) => {
                    let edits = vec![TextEdit {
                        range: Range::new(Position::new(0, 0), end),
                        new_text,
                    }];
                    connection.sender.send(Response::new_ok(id, edits).into())?;
                }
                Ok(Outcome::Unchanged) => finish(connection, id, None)?,
                Ok(Outcome::Skipped(reason)) => finish(connection, id, Some(reason))?,
                Err(reason) => finish(connection, id, Some(format!("{reason:?}")))?,
            }
        }
        Ok(())
    }

    pub fn cancel(
        &mut self,
        connection: &Connection,
        params: serde_json::Value,
    ) -> anyhow::Result<()> {
        #[derive(Deserialize)]
        struct Cancellation {
            id: RequestId,
        }
        let cancellation: Cancellation = serde_json::from_value(params)?;
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.id == cancellation.id)
        {
            self.stop(connection)?;
        }
        Ok(())
    }

    pub fn stop(&mut self, connection: &Connection) -> anyhow::Result<()> {
        if let Some(pending) = self.pending.take() {
            let Pending { id, worker, .. } = pending;
            drop(worker);
            connection
                .sender
                .send(Response::new_err(id, -32800, "formatting cancelled".into()).into())?;
        }
        Ok(())
    }
}

fn finish(connection: &Connection, id: RequestId, reason: Option<String>) -> anyhow::Result<()> {
    if let Some(reason) = reason {
        notifications::log(
            connection,
            json!({"event":"formatting.skipped", "id":id, "reason":reason}).to_string(),
        )?;
    }
    connection
        .sender
        .send(Response::new_ok(id, Vec::<TextEdit>::new()).into())?;
    Ok(())
}
