use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::worker_protocol::{self as wire, Outcome, Ready};

#[cfg(all(test, unix))]
#[path = "worker_tests.rs"]
mod tests;

pub struct Program {
    pub executable: PathBuf,
    pub arguments: Vec<OsString>,
}

impl Program {
    pub fn current() -> std::io::Result<Self> {
        Ok(Self {
            executable: std::env::current_exe()?,
            arguments: vec!["--format-worker".into()],
        })
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.executable);
        command.args(&self.arguments);
        command
    }
}

#[derive(Clone, Copy)]
pub struct Budget {
    pub startup: Duration,
    pub computation: Duration,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            startup: Duration::from_secs(45),
            computation: Duration::from_secs(2),
        }
    }
}

#[derive(Debug)]
pub enum Failure {
    InputLimit,
    Spawn,
    Transport,
    StartupTimeout,
    ComputationTimeout,
    Crash,
    Busy,
    Stale,
}

enum Event {
    Ready(Instant),
    Finished(Result<Outcome, Failure>, Instant),
}

pub struct Worker {
    child: Child,
    transport: Option<JoinHandle<()>>,
    events: Receiver<Event>,
    deadline: Instant,
    ready: bool,
    budget: Budget,
}

impl Worker {
    pub fn spawn(mut command: Command, source: String, budget: Budget) -> Result<Self, Failure> {
        if source.len() > wire::INPUT_LIMIT {
            return Err(Failure::InputLimit);
        }
        let child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| Failure::Spawn)?;
        let (sender, events) = mpsc::sync_channel(2);
        let mut worker = Self {
            child,
            transport: None,
            events,
            deadline: Instant::now() + budget.startup,
            ready: false,
            budget,
        };
        let mut input = worker.child.stdin.take().ok_or(Failure::Transport)?;
        let mut output = worker.child.stdout.take().ok_or(Failure::Transport)?;
        worker.transport = Some(
            std::thread::Builder::new()
                .name("iris-format-transport".into())
                .spawn(move || {
                    let result = (|| {
                        wire::read::<Ready>(&mut output, 32).map_err(|_| Failure::Transport)?;
                        sender
                            .try_send(Event::Ready(Instant::now()))
                            .map_err(|_| Failure::Transport)?;
                        wire::write(&mut input, &source, wire::REQUEST_LIMIT)
                            .map_err(|_| Failure::Transport)?;
                        drop(input);
                        wire::read(&mut output, wire::OUTPUT_LIMIT).map_err(|_| Failure::Transport)
                    })();
                    let _ = sender.try_send(Event::Finished(result, Instant::now()));
                })
                .map_err(|_| Failure::Spawn)?,
        );
        Ok(worker)
    }

    pub fn poll(&mut self) -> Option<Result<Outcome, Failure>> {
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Ready(at) => {
                    if at > self.deadline {
                        return Some(Err(Failure::StartupTimeout));
                    }
                    self.ready = true;
                    self.deadline = at + self.budget.computation;
                }
                Event::Finished(result, at) => {
                    return Some(if at > self.deadline {
                        Err(self.timeout())
                    } else {
                        result
                    });
                }
            }
        }
        if Instant::now() >= self.deadline {
            return Some(Err(self.timeout()));
        }
        match self.child.try_wait() {
            Ok(Some(status)) if !status.success() => Some(Err(Failure::Crash)),
            Err(_) => Some(Err(Failure::Transport)),
            Ok(Some(_) | None) => None,
        }
    }

    const fn timeout(&self) -> Failure {
        if self.ready {
            Failure::ComputationTimeout
        } else {
            Failure::StartupTimeout
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Err(error) = self.child.kill()
            && error.kind() != std::io::ErrorKind::InvalidInput
        {
            eprintln!("format.worker.kill: {error}");
        }
        if let Err(error) = self.child.wait() {
            eprintln!("format.worker.reap: {error}");
        }
        if let Some(transport) = self.transport.take()
            && transport.join().is_err()
        {
            eprintln!("format.worker.transport: thread panicked");
        }
    }
}
