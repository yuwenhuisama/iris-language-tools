use super::{Connection, Ordering, Pending, Response, Semantics};

impl Semantics {
    pub fn notification(&mut self, notification: lsp_server::Notification) -> anyhow::Result<()> {
        match notification.method.as_str() {
            "workspace/didChangeWorkspaceFolders" => {
                let params: lsp_types::DidChangeWorkspaceFoldersParams =
                    serde_json::from_value(notification.params)?;
                self.roots
                    .retain(|uri| !params.event.removed.iter().any(|folder| &folder.uri == uri));
                for folder in params.event.added {
                    if !self.roots.contains(&folder.uri) {
                        self.roots.push(folder.uri);
                    }
                }
                self.changed()?;
            }
            "workspace/didChangeWatchedFiles" => {
                let params: lsp_types::DidChangeWatchedFilesParams =
                    serde_json::from_value(notification.params)?;
                if !params.changes.is_empty() {
                    self.changed()?;
                }
            }
            "textDocument/didSave" => {
                let _: lsp_types::DidSaveTextDocumentParams =
                    serde_json::from_value(notification.params)?;
                self.changed()?;
            }
            _ => {}
        }
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
    pub(super) fn finish_error(
        &mut self,
        connection: &Connection,
        error: (i32, &str),
    ) -> anyhow::Result<()> {
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
