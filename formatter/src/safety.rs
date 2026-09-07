use crate::SkipReason;

pub const INPUT_LIMIT: usize = 256 * 1024;
pub const OUTPUT_LIMIT: usize = 1024 * 1024;
pub const NESTING_LIMIT: usize = 128;

pub const fn check_source(source: &str) -> Result<(), SkipReason> {
    if source.len() > INPUT_LIMIT {
        return Err(SkipReason::InputLimit);
    }
    Ok(())
}
