mod layout;
mod normalize;
mod normalize_expression;
#[cfg(test)]
mod normalize_tests;
mod preflight;
mod safety;
mod spacing;
mod tokens;
mod tree;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkipReason {
    InputLimit,
    OutputLimit,
    NestingLimit,
    Continuation,
    LexicalDiagnostics,
    MismatchedDelimiter,
    UnclosedDelimiter,
    UncertainProtectedRange,
    ParseDiagnostics,
    CandidateParseDiagnostics,
    SemanticMismatch,
    TokenMismatch,
}

#[derive(Debug, Eq, PartialEq)]
pub enum FormatOutcome {
    Changed(String),
    Unchanged,
    Skipped(SkipReason),
}

/// Formats Iris with the fixed official two-space, 120-column style.
#[must_use]
pub fn format_document(source: &str) -> FormatOutcome {
    match format_checked(source) {
        Ok(output) if output == source => FormatOutcome::Unchanged,
        Ok(output) => FormatOutcome::Changed(output),
        Err(reason) => FormatOutcome::Skipped(reason),
    }
}

fn format_checked(source: &str) -> Result<String, SkipReason> {
    safety::check_source(source)?;
    let pieces = tokens::scan(source)?;
    let tree = tree::build(&pieces)?;
    preflight::syntax(&tree)?;
    let original = iris_parser::parse(source);
    if !original.is_clean() || !original.program_accepted {
        return Err(SkipReason::ParseDiagnostics);
    }
    let output = layout::render(&tree)?;
    let candidate = iris_parser::parse(&output);
    if !candidate.is_clean() || !candidate.program_accepted {
        return Err(SkipReason::CandidateParseDiagnostics);
    }
    if normalize::program(original.program) != normalize::program(candidate.program) {
        return Err(SkipReason::SemanticMismatch);
    }
    tokens::verify(&pieces, &tokens::scan(&output)?)?;
    Ok(output)
}
