use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, SyncSender},
};
use std::thread::JoinHandle;

use iris_analysis::{AnalysisSnapshot, FileId, GroupId, SourceInput};
use lsp_types::{CompletionItem, Uri};
use serde_json::Value;

use crate::{
    semantic_query::{Query, QueryError},
    workspace::{GroupIdentity, OverlaySet, Snapshot, Workspace},
};

#[cfg(test)]
#[path = "semantic_cache_tests.rs"]
mod tests;

pub const CAPACITY: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Epoch(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ticket(pub u64);

pub struct Inputs {
    pub epoch: Epoch,
    pub roots: Vec<Uri>,
    pub overlays: OverlaySet,
}

pub struct Job {
    pub ticket: Ticket,
    pub inputs: Arc<Inputs>,
    pub query: Query,
    pub cancelled: Arc<AtomicBool>,
}

pub struct Finished {
    pub ticket: Ticket,
    pub result: Result<Value, Failure>,
}

#[derive(Debug, thiserror::Error)]
pub enum Failure {
    #[error("semantic request cancelled")]
    Cancelled,
    #[error(transparent)]
    Query(#[from] QueryError),
    #[error(transparent)]
    Workspace(#[from] crate::workspace::WorkspaceError),
    #[error("workspace identity capacity exceeded")]
    Identity,
}

pub struct SemanticWorker {
    pub sender: Option<SyncSender<Job>>,
    pub results: Receiver<Finished>,
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl SemanticWorker {
    #[cfg(test)]
    pub fn controlled() -> (Self, Receiver<Job>, SyncSender<Finished>) {
        let (sender, jobs) = mpsc::sync_channel(CAPACITY);
        let (finished, results) = mpsc::sync_channel(CAPACITY);
        (
            Self {
                sender: Some(sender),
                results,
                stopping: Arc::new(AtomicBool::new(false)),
                thread: None,
            },
            jobs,
            finished,
        )
    }

    pub fn new(keywords: Vec<CompletionItem>) -> std::io::Result<Self> {
        let (sender, jobs) = mpsc::sync_channel::<Job>(CAPACITY);
        let (finished, results) = mpsc::sync_channel(CAPACITY);
        let stopping = Arc::new(AtomicBool::new(false));
        let stop = Arc::clone(&stopping);
        let thread = std::thread::Builder::new()
            .name("iris-semantics".into())
            .spawn(move || {
                let mut workspace = Workspace::new(Vec::new());
                let mut roots = Vec::new();
                let mut cache = None;
                while let Ok(job) = jobs.recv() {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let result = execute(&job, (&mut workspace, &mut roots, &mut cache), &keywords);
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    if finished
                        .send(Finished {
                            ticket: job.ticket,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })?;
        Ok(Self {
            sender: Some(sender),
            results,
            stopping,
            thread: Some(thread),
        })
    }

    pub fn stop(&mut self) {
        self.stopping.store(true, Ordering::Relaxed);
        self.sender.take();
    }
}

impl Drop for SemanticWorker {
    fn drop(&mut self) {
        self.stop();
        if let Some(thread) = self.thread.take()
            && thread.is_finished()
        {
            let _result = thread.join();
        }
    }
}

struct Cache {
    epoch: Epoch,
    snapshot: Snapshot,
    analysis: AnalysisSnapshot,
}

fn execute(
    job: &Job,
    state: (&mut Workspace, &mut Vec<Uri>, &mut Option<Cache>),
    keywords: &[CompletionItem],
) -> Result<Value, Failure> {
    let (workspace, roots, cache) = state;
    check_cancelled(job)?;
    if cache
        .as_ref()
        .is_none_or(|cache| cache.epoch != job.inputs.epoch)
    {
        if *roots == job.inputs.roots {
            workspace.reload(&job.inputs.overlays)?;
        } else {
            workspace.replace_roots(job.inputs.roots.clone(), &job.inputs.overlays)?;
            roots.clone_from(&job.inputs.roots);
        }
        check_cancelled(job)?;
        let snapshot = workspace.snapshot();
        let mut groups: Vec<GroupIdentity> = Vec::new();
        let mut sources = Vec::new();
        for (index, file) in snapshot.files.iter().enumerate() {
            check_cancelled(job)?;
            let group = groups
                .iter()
                .position(|group| group == &file.group)
                .unwrap_or_else(|| {
                    groups.push(file.group.clone());
                    groups.len() - 1
                });
            sources.push(SourceInput {
                id: FileId(u32::try_from(index).map_err(|_| Failure::Identity)?),
                group: GroupId(u32::try_from(group).map_err(|_| Failure::Identity)?),
                text: Arc::clone(&file.text),
            });
        }
        let incomplete_groups = if snapshot.is_complete() {
            Vec::new()
        } else {
            sources.iter().map(|source| source.group).collect()
        };
        let analysis = AnalysisSnapshot::new(
            sources
                .into_iter()
                .take_while(|_| !job.cancelled.load(Ordering::Relaxed)),
        )
        .with_incomplete_groups(incomplete_groups);
        check_cancelled(job)?;
        *cache = Some(Cache {
            epoch: job.inputs.epoch,
            snapshot,
            analysis,
        });
    }
    let cache = cache.as_ref().ok_or(Failure::Cancelled)?;
    check_cancelled(job)?;
    let result = job
        .query
        .execute((&cache.snapshot, &cache.analysis), keywords)?;
    check_cancelled(job)?;
    Ok(result)
}

fn check_cancelled(job: &Job) -> Result<(), Failure> {
    if job.cancelled.load(Ordering::Relaxed) {
        Err(Failure::Cancelled)
    } else {
        Ok(())
    }
}
