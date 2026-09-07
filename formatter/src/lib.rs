mod layout;
mod safety;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormatOptions {
    tab_size: u8,
    insert_spaces: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidTabSize(pub u32);

impl FormatOptions {
    /// Constructs indentation options.
    ///
    /// # Errors
    /// Returns `InvalidTabSize` unless `tab_size` is between 1 and 16.
    pub fn new(tab_size: u32, insert_spaces: bool) -> Result<Self, InvalidTabSize> {
        if !(1..=16).contains(&tab_size) {
            return Err(InvalidTabSize(tab_size));
        }
        Ok(Self {
            tab_size: u8::try_from(tab_size).map_err(|_| InvalidTabSize(tab_size))?,
            insert_spaces,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkipReason {
    InputLimit,
    OutputLimit,
    NestingLimit,
    UnsupportedLiteral,
    Continuation,
    LexicalDiagnostics,
    MismatchedDelimiter,
    UnclosedDelimiter,
    UncertainProtectedRange,
    TokenMismatch,
}

#[derive(Debug, Eq, PartialEq)]
pub enum FormatOutcome {
    Changed(String),
    Unchanged,
    Skipped(SkipReason),
}

#[must_use]
pub fn format_document(source: &str, options: FormatOptions) -> FormatOutcome {
    match format_checked(source, options) {
        Ok(output) if output == source => FormatOutcome::Unchanged,
        Ok(output) => FormatOutcome::Changed(output),
        Err(reason) => FormatOutcome::Skipped(reason),
    }
}

fn format_checked(source: &str, options: FormatOptions) -> Result<String, SkipReason> {
    safety::check_source(source)?;
    let lexed = iris_lexer::lex(source.as_bytes());
    if !lexed.is_clean() {
        return Err(SkipReason::LexicalDiagnostics);
    }
    let (output, mapped) = layout::indent(source, lexed.tokens(), options)?;
    let checked = iris_lexer::lex(output.as_bytes());
    if !checked.is_clean() || checked.tokens() != mapped {
        return Err(SkipReason::TokenMismatch);
    }
    Ok(output)
}
