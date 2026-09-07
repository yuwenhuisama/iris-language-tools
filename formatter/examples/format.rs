use iris_formatter::{FormatOutcome, format_document};

fn main() {
    let source = std::env::args().nth(1).unwrap_or_default();
    match format_document(&source) {
        FormatOutcome::Changed(output) => print!("{output}"),
        FormatOutcome::Unchanged => print!("{source}"),
        FormatOutcome::Skipped(reason) => {
            eprintln!("Formatting skipped: {reason:?}");
            std::process::exit(1);
        }
    }
}
