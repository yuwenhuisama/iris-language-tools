use std::io::{Read, Write};

use iris_formatter::{FormatOutcome, format_document};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

pub const INPUT_LIMIT: usize = 256 * 1024;
pub const OUTPUT_LIMIT: usize = 1024 * 1024;
pub const REQUEST_LIMIT: usize = INPUT_LIMIT * 6 + 32;

#[derive(Debug, Deserialize, Serialize)]
pub enum Outcome {
    Changed(String),
    Unchanged,
    Skipped(String),
}

#[derive(Deserialize, Serialize)]
pub enum Ready {
    Ready,
}

pub fn read<T: DeserializeOwned>(reader: &mut impl Read, limit: usize) -> anyhow::Result<T> {
    let mut header = [0; 4];
    reader.read_exact(&mut header)?;
    let length = usize::try_from(u32::from_be_bytes(header))?;
    anyhow::ensure!(length <= limit, "worker frame exceeds limit");
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn write<T: Serialize>(writer: &mut impl Write, value: &T, limit: usize) -> anyhow::Result<()> {
    let bytes = serde_json::to_vec(value)?;
    anyhow::ensure!(bytes.len() <= limit, "worker frame exceeds limit");
    writer.write_all(&u32::try_from(bytes.len())?.to_be_bytes())?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    Ok(())
}

pub fn run() -> anyhow::Result<()> {
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    write(&mut output, &Ready::Ready, 32)?;
    let source: String = read(&mut input, REQUEST_LIMIT)?;
    drop(input);
    anyhow::ensure!(source.len() <= INPUT_LIMIT, "worker input exceeds limit");
    let outcome = match format_document(&source) {
        FormatOutcome::Changed(text) => Outcome::Changed(text),
        FormatOutcome::Unchanged => Outcome::Unchanged,
        FormatOutcome::Skipped(reason) => Outcome::Skipped(format!("{reason:?}")),
    };
    write(&mut output, &outcome, OUTPUT_LIMIT)
}
