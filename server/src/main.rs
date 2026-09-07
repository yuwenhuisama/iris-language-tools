#[cfg(test)]
mod diagnostic_tests;
mod diagnostics;
mod documents;
mod formatting;
#[cfg(all(test, unix))]
mod formatting_tests;
mod notifications;
mod positions;
mod semantic_query;
mod semantic_worker;
mod semantics;
mod session;
mod worker;
mod worker_protocol;
mod workspace;

use std::process::ExitCode;

fn main() -> anyhow::Result<ExitCode> {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|argument| argument == "--format-worker")
    {
        worker_protocol::run()?;
        return Ok(ExitCode::SUCCESS);
    }
    let (connection, threads) = lsp_server::Connection::stdio();
    let status = session::run(&connection)?;
    drop(connection);
    threads.join()?;
    Ok(status)
}
