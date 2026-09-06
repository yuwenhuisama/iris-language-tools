#[cfg(test)]
mod diagnostic_tests;
mod diagnostics;
mod documents;
mod notifications;
mod positions;
mod session;

use std::process::ExitCode;

fn main() -> anyhow::Result<ExitCode> {
    let (connection, threads) = lsp_server::Connection::stdio();
    let status = session::run(&connection)?;
    drop(connection);
    threads.join()?;
    Ok(status)
}
