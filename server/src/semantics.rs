use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::TryRecvError,
};

use lsp_server::{Connection, Request, RequestId, Response};
use lsp_types::{CompletionItem, Uri};
use serde::Deserialize;

use crate::{
    documents::Documents,
    notifications,
    semantic_query::{Query, QueryError},
    semantic_worker::{CAPACITY, Epoch, Failure, Inputs, Job, SemanticWorker, Ticket},
    workspace::OverlaySet,
};

#[path = "semantic_lifecycle.rs"]
mod lifecycle;

#[cfg(test)]
#[path = "semantic_tests.rs"]
mod tests;

struct Pending {
    ticket: Ticket,
    id: Option<RequestId>,
    uri: Uri,
    epoch: Epoch,
    document: Option<(i32, Arc<()>)>,
    cancelled: Arc<AtomicBool>,
}

impl Pending {
    fn current(&self, epoch: Epoch, documents: &Documents) -> bool {
        self.epoch == epoch
            && match (&self.document, documents.get(&self.uri)) {
                (Some((version, generation)), Some(document)) => {
                    *version == document.version && Arc::ptr_eq(generation, &document.generation)
                }
                (None, None) => true,
                (Some(_), None) | (None, Some(_)) => false,
            }
    }
}

pub struct Semantics {
    worker: SemanticWorker,
    pending: Vec<Pending>,
    epoch: Epoch,
    next_ticket: u64,
    pub roots: Vec<Uri>,
    inputs: Option<Arc<Inputs>>,
}

impl Semantics {
    pub fn new(keywords: Vec<CompletionItem>) -> std::io::Result<Self> {
        Ok(Self {
            worker: SemanticWorker::new(keywords)?,
            pending: Vec::new(),
            epoch: Epoch(0),
            next_ticket: 0,
            roots: Vec::new(),
            inputs: None,
        })
    }

    pub fn changed(&mut self) -> anyhow::Result<()> {
        self.epoch = Epoch(
            self.epoch
                .0
                .checked_add(1)
                .ok_or_else(|| anyhow::anyhow!("semantic epoch exhausted"))?,
        );
        self.inputs = None;
        Ok(())
    }

    pub fn request(
        &mut self,
        request: Request,
        context: (&Connection, &Documents),
    ) -> anyhow::Result<()> {
        let (connection, documents) = context;
        let query = match Query::decode(&request.method, request.params) {
            Ok(query) => query,
            Err(error) => {
                connection
                    .sender
                    .send(Response::new_err(request.id, -32602, error.to_string()).into())?;
                return Ok(());
            }
        };
        if self.pending.len() == CAPACITY {
            connection.sender.send(
                Response::new_err(request.id, -32802, "semantic worker is busy".into()).into(),
            )?;
            return Ok(());
        }
        let inputs = Arc::clone(self.inputs.get_or_insert_with(|| {
            Arc::new(Inputs {
                epoch: self.epoch,
                roots: self.roots.clone(),
                overlays: OverlaySet::capture(documents),
            })
        }));
        let ticket = Ticket(self.next_ticket);
        self.next_ticket = self
            .next_ticket
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("semantic ticket exhausted"))?;
        let pending = Pending {
            ticket,
            id: Some(request.id),
            uri: query.uri.clone(),
            epoch: self.epoch,
            document: documents
                .get(&query.uri)
                .map(|document| (document.version, Arc::clone(&document.generation))),
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        let job = Job {
            ticket,
            inputs,
            query,
            cancelled: Arc::clone(&pending.cancelled),
        };
        let sent = self
            .worker
            .sender
            .as_ref()
            .is_some_and(|sender| sender.try_send(job).is_ok());
        if sent {
            self.pending.push(pending);
        } else if let Some(id) = pending.id {
            connection
                .sender
                .send(Response::new_err(id, -32803, "semantic worker unavailable".into()).into())?;
        }
        Ok(())
    }

    pub fn poll(&mut self, connection: &Connection, documents: &Documents) -> anyhow::Result<()> {
        for pending in &mut self.pending {
            if !pending.current(self.epoch, documents) {
                pending.finish_error(connection, (-32801, "content modified"))?;
            }
        }
        loop {
            let finished = match self.worker.results.try_recv() {
                Ok(finished) => finished,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    for pending in &mut self.pending {
                        pending
                            .finish_error(connection, (-32803, "semantic worker unavailable"))?;
                    }
                    self.pending.clear();
                    break;
                }
            };
            let Some(index) = self
                .pending
                .iter()
                .position(|pending| pending.ticket == finished.ticket)
            else {
                continue;
            };
            let pending = self.pending.remove(index);
            let Some(id) = pending.id else {
                continue;
            };
            let response = match finished.result {
                Ok(result) => Response::new_ok(id, result),
                Err(error) => {
                    let code = match error {
                        Failure::Cancelled => -32800,
                        Failure::Query(QueryError::Position) => -32602,
                        Failure::Query(QueryError::Incomplete | QueryError::Target)
                        | Failure::Workspace(_)
                        | Failure::Identity => -32803,
                    };
                    if code == -32803 {
                        notifications::log(connection, serde_json::json!({"event":"semantic.failed","id":id,"reason":error.to_string()}).to_string())?;
                    }
                    Response::new_err(id, code, error.to_string())
                }
            };
            connection.sender.send(response.into())?;
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
        let params: Cancellation = serde_json::from_value(params)?;
        for pending in &mut self.pending {
            if pending.id.as_ref() == Some(&params.id) {
                pending.finish_error(connection, (-32800, "semantic request cancelled"))?;
            }
        }
        Ok(())
    }

    pub fn stop(&mut self, connection: &Connection) -> anyhow::Result<()> {
        for pending in &mut self.pending {
            pending.finish_error(connection, (-32800, "semantic request cancelled"))?;
        }
        self.worker.stop();
        Ok(())
    }
}

impl Drop for Semantics {
    fn drop(&mut self) {
        for pending in &self.pending {
            pending.cancelled.store(true, Ordering::Relaxed);
        }
        self.worker.stop();
    }
}

impl Pending {
    fn finish_error(&mut self, connection: &Connection, error: (i32, &str)) -> anyhow::Result<()> {
        let (code, message) = error;
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(id) = self.id.take() {
            connection
                .sender
                .send(Response::new_err(id, code, message.into()).into())?;
        }
        Ok(())
    }
}
