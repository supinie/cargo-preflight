use colored::Colorize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PreflightError {
    /// Invalid entry in `checks` in Preflight config, see [valid options](index.html#possible-options)
    #[error("{}{config}", "Invalid check in config: ".red())]
    InvalidCheck { config: String },

    /// Invalid entry in `run_when` in Preflight config, see [valid options](index.html#possible-options)
    #[error("{}{config}", "Invalid hook in config: ".red())]
    InvalidHook { config: String },

    /// `cargo fmt --check` preflight check failed
    #[error("    {}{fmt_output}", "[x] Formatting preflight check failed".red().bold())]
    FormatFailed { fmt_output: String },

    /// `cargo clippy -- -D warnings` preflight check failed
    #[error("    {}{clippy_output}", "[x] Clippy preflight check failed:\n".red().bold())]
    ClippyFailed { clippy_output: String },

    /// `cargo check --tests` preflight check failed
    #[error("    {}{check_outputs}", "[x] Check test preflight check failed:\n".red().bold())]
    CheckTestsFailed { check_outputs: String },

    /// `cargo check --examples` preflight check failed
    #[error("    {}{check_outputs}", "[x] Check examples preflight check failed:\n".red().bold())]
    CheckExamplesFailed { check_outputs: String },

    /// `cargo check --benches` preflight check failed
    #[error("    {}{check_outputs}", "[x] Check benches preflight check failed:\n".red().bold())]
    CheckBenchesFailed { check_outputs: String },

    /// `cargo test` preflight check failed
    #[error("    {}{test_outputs}", "[x] Test preflight check failed:\n".red().bold())]
    TestsFailed { test_outputs: String },

    /// `cargo shear` preflight check failed
    #[error("    {}{shear_output}", "[x] Unused dependencies preflight check failed:\n".red().bold())]
    ShearFailed { shear_output: String },

    #[error("    {}{failed_check}", "Preflight ended due to failed check: ".red().bold())]
    OverrideCancelled { failed_check: String },
}

impl From<PreflightError> for std::io::Error {
    fn from(err: PreflightError) -> Self {
        Self::other(err)
    }
}

pub fn failed_check_index(checks: &[String], error: &PreflightError) -> Option<usize> {
    // Map each error variant to the corresponding check name
    let failed_check = match error {
        PreflightError::FormatFailed { .. } => "fmt",
        PreflightError::ClippyFailed { .. } => "clippy",
        PreflightError::CheckTestsFailed { .. } => "check_tests",
        PreflightError::CheckExamplesFailed { .. } => "check_examples",
        PreflightError::CheckBenchesFailed { .. } => "check_benches",
        PreflightError::TestsFailed { .. } => "test",
        PreflightError::ShearFailed { .. } => "unused_deps",
        PreflightError::InvalidCheck { config } => config.as_str(),
        PreflightError::InvalidHook { .. } => "hook",
        PreflightError::OverrideCancelled { .. } => unreachable!(),
    };

    // Find the index of the failed check in the checks vector
    checks.iter().position(|check| check == failed_check)
}
