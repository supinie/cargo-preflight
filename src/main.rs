use serde_derive::{Deserialize, Serialize};
use std::fs::exists;
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
struct PreflightConfig {
    run_when: Vec<String>,
    remote_branches: Vec<String>,
    preflight_checks: Vec<String>,
}

impl std::default::Default for PreflightConfig {
    fn default() -> Self {
        Self {
            run_when: vec!["push".into()],
            remote_branches: vec!["main".into(), "master".into()],
            preflight_checks: vec!["fmt".into(), "test".into()],
        }
    }
}

#[derive(Debug, Error)]
pub enum PreflightError {
    #[error("`cargo fmt` failed")]
    FormatError(#[source] cargo::util::errors::CliError),
}

fn check_local_config() -> Result<PreflightConfig, confy::ConfyError> {
    if exists("./.cargo-preflight.toml").expect("Can't check for local config") {
        confy::load_path("./cargo-preflight.toml")
    } else {
        confy::load("cargo-preflight", None)
    }
}

fn run_preflight_checks() -> Result<(), PreflightError> {
    let _ = cargo::ops::run_tests()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = check_local_config()?;
    dbg!(cfg);
    Ok(())
}
