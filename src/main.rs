#![forbid(unsafe_code)]
#![warn(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::checked_conversions,
    clippy::implicit_saturating_sub,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used,
    clippy::pedantic,
    clippy::nursery,
    rust_2018_idioms,
    unused_lifetimes,
    unused_qualifications
)]
#![allow(clippy::too_long_first_doc_paragraph)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! # About
//!
//! Preflight is a custom Cargo subcommand to run local "CI" on certain Git actions.
//!
//! Preflight's aim is to ensure that trivially broken commits don't reach your remote, wasting CI time, adding extra fix commits, and most importantly saving you the embarrasment.
//!
//! Preflight runs as a git hook to automatically run checks on commit or push.
//!
//! # Installing Preflight
//!
//! _coming soon: `cargo add cargo-preflight`_
//!
//! # Using Preflight
//!
//! Preflight can be enabled in a repository by running:
//!
//! ```sh
//! cargo preflight --init
//! ```
//!
//! # Configuring Preflight
//!
//! Preflight can be configured by running:
//!
//! ```sh
//! cargo preflight --config
//! ```
//!
//! which will open a configuration wizard to walk you through the available options.
//!
//! Alteratively, Preflight can be manually configured by editing the global `~/.config/cargo-preflight/preflight.toml` configuration or local `<your repo>/.preflight.toml` configuration files.
//!
//! ## Possible Options
//!
//! ```toml
//! run_when = [
//!     "commit",
//!     "push",
//! ] # Default values: ["push"]
//!
//! checks = [
//!     "fmt",
//!     "clippy",
//!     "test",
//!     "check_tests",
//!     "check_examples",
//!     "check_benches",
//! ] # Default values: ["fmt", "test"]
//! ```

use anyhow::Result;
use colored::Colorize;
use inquire::{MultiSelect, Select};
use serde_derive::{Deserialize, Serialize};
use std::{env, fs::exists, process::Command};
use thiserror::Error;

const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);

#[derive(Debug, Serialize, Deserialize)]
struct PreflightConfig {
    run_when: Vec<String>,
    // remote_branches: Vec<String>,
    checks: Vec<String>,
}

impl Default for PreflightConfig {
    fn default() -> Self {
        Self {
            run_when: vec!["push".into()],
            // remote_branches: vec!["main".into(), "master".into()],
            checks: vec!["fmt".into(), "test".into()],
        }
    }
}

#[derive(Error, Debug)]
pub enum PreflightError {
    /// Invalid entry in `checks` in Preflight config, see [valid options](index.html#possible-options)
    #[error("{}{config}", "Invalid check in config: ".red())]
    InvalidCheck { config: String },

    /// Invalid entry in `run_when` in Preflight config, see [valid options](index.html#possible-options)
    #[error("{}{config}", "Invalid hook in config: ".red())]
    InvalidHook { config: String },

    /// `cargo fmt --check` preflight check failed
    #[error("\n    {}{fmt_output}", "[x] Formatting preflight check failed".red().bold())]
    FormatFailed { fmt_output: String },

    /// `cargo clippy -- -D warnings` preflight check failed
    #[error("\n    {}{clippy_output}", "[x] Clippy preflight check failed:\n".red().bold())]
    ClippyFailed { clippy_output: String },

    /// `cargo check --tests` preflight check failed
    #[error("\n    {}{check_outputs}", "[x] Check test preflight check failed:\n".red().bold())]
    CheckTestsFailed { check_outputs: String },

    /// `cargo check --examples` preflight check failed
    #[error("\n    {}{check_outputs}", "[x] Check examples preflight check failed:\n".red().bold())]
    CheckExamplesFailed { check_outputs: String },

    /// `cargo check --benches` preflight check failed
    #[error("\n    {}{check_outputs}", "[x] Check benches preflight check failed:\n".red().bold())]
    CheckBenchesFailed { check_outputs: String },

    /// `cargo test` preflight check failed
    #[error("\n    {}{test_outputs}", "[x] Test preflight check failed:\n".red().bold())]
    TestsFailed { test_outputs: String },
}

impl From<PreflightError> for std::io::Error {
    fn from(err: PreflightError) -> Self {
        Self::other(err)
    }
}

fn check_local_config() -> Result<PreflightConfig, confy::ConfyError> {
    if exists("./.preflight.toml").expect("Can't check for local config") {
        confy::load_path("./.preflight.toml")
    } else {
        confy::load("cargo-preflight", "preflight")
    }
}

fn preflight_checks(cfg: PreflightConfig) -> Result<()> {
    // get current workspace for tests and fmt
    for check in cfg.checks {
        match check.as_str() {
            "fmt" => cargo_fmt(),
            "clippy" => cargo_clippy(),
            "check_tests" => cargo_check_tests(),
            "check_examples" => cargo_check_examples(),
            "check_benches" => cargo_check_benches(),
            "test" => cargo_test(),
            _ => Err(PreflightError::InvalidCheck { config: check }.into()),
        }?;
    }
    Ok(())
}

fn cargo_fmt() -> Result<()> {
    let output = Command::new("cargo")
        .arg("fmt")
        .arg("--")
        .arg("--check") // This ensures no changes are made, only checks the formatting
        .output()?;

    if output.status.success() {
        println!("    {}", "[√] Formatting preflight check passed".green());
        Ok(())
    } else {
        Err(PreflightError::FormatFailed {
            fmt_output: String::from_utf8_lossy(&output.stdout).to_string(),
        }
        .into())
    }
}

fn cargo_clippy() -> Result<()> {
    let output = Command::new("cargo")
        .arg("clippy")
        .arg("--")
        .args(["-D", "warnings"])
        .output()?;

    if output.status.success() {
        println!("    {}", "[√] Clippy preflight check passed".green());
        Ok(())
    } else {
        Err(PreflightError::ClippyFailed {
            clippy_output: String::from_utf8_lossy(&output.stderr).to_string(),
        }
        .into())
    }
}

fn cargo_check_tests() -> Result<()> {
    let output = Command::new("cargo").arg("check").arg("--tests").output()?;

    if output.status.success() {
        println!("{}", "    [√] Check tests preflight check passed".green());
        Ok(())
    } else {
        Err(PreflightError::CheckTestsFailed {
            check_outputs: String::from_utf8_lossy(&output.stdout).to_string(),
        }
        .into())
    }
}

fn cargo_check_examples() -> Result<()> {
    let output = Command::new("cargo")
        .arg("check")
        .arg("--examples")
        .output()?;

    if output.status.success() {
        println!(
            "{}",
            "    [√] Check examples preflight check passed".green()
        );
        Ok(())
    } else {
        Err(PreflightError::CheckExamplesFailed {
            check_outputs: String::from_utf8_lossy(&output.stdout).to_string(),
        }
        .into())
    }
}
fn cargo_check_benches() -> Result<()> {
    let output = Command::new("cargo")
        .arg("check")
        .arg("--benches")
        .output()?;

    if output.status.success() {
        println!("{}", "    [√] Check benches preflight check passed".green());
        Ok(())
    } else {
        Err(PreflightError::CheckBenchesFailed {
            check_outputs: String::from_utf8_lossy(&output.stdout).to_string(),
        }
        .into())
    }
}

fn cargo_test() -> Result<()> {
    let output = Command::new("cargo").arg("test").output()?;

    if output.status.success() {
        println!("{}", "    [√] Tests preflight check passed".green());
        Ok(())
    } else {
        Err(PreflightError::TestsFailed {
            test_outputs: String::from_utf8_lossy(&output.stdout).to_string(),
        }
        .into())
    }
}

fn init_symlink(cfg: PreflightConfig) -> Result<()> {
    let mut path = dirs::home_dir().expect("No valid home dir found");
    path.push(".cargo/bin/cargo-preflight");
    for hook in cfg.run_when {
        match hook.as_str() {
            "commit" => std::os::unix::fs::symlink(&path, "./.git/hooks/pre-commit"),
            "push" => std::os::unix::fs::symlink(&path, "./.git/hooks/pre-push"),
            _ => Err(PreflightError::InvalidHook { config: hook }.into()),
        }?;
    }
    Ok(())
}

#[allow(clippy::cognitive_complexity)]
fn cargo_subcommand<I: Iterator<Item = String>>(args: I) -> clap::ArgMatches {
    let cmd = clap::Command::new("cargo")
        .bin_name("cargo")
        .styles(CLAP_STYLING)
        .subcommand_required(true)
        .subcommand(
            clap::command!("preflight")
                .arg(clap::arg!(--"init" "Initialise preflight in the current repository. This will add git hooks depending on local/global config (priority in that order)").value_parser(clap::value_parser!(bool)))
                .arg(clap::arg!(<REMOTE>).value_parser(clap::value_parser!(String)))
                .arg(clap::arg!(--"config" "Configure preflight checks to run").value_parser(clap::value_parser!(bool))),
        );
    match cmd.get_matches_from(args).subcommand() {
        Some(("preflight", matches)) => matches.clone(),
        _ => unreachable!("clap should ensure we don't get here"),
    }
}

fn standalone_command<I: Iterator<Item = String>>(args: I) -> clap::ArgMatches {
    let cmd = clap::Command::new("cargo-preflight")
        .styles(CLAP_STYLING)
        .arg(clap::arg!(--"init" "Initialise preflight in the current repository. This will add git hooks depending on local/global config (priority in that order)").value_parser(clap::value_parser!(bool)))
        .arg(clap::Arg::new("REMOTE").hide(true))
        .arg(clap::arg!(--"config" "Configure preflight checks to run").value_parser(clap::value_parser!(bool)));
    cmd.get_matches_from(args)
}

fn update_config() -> Result<()> {
    let config_types = vec!["global", "local"];
    let checks = vec![
        "fmt",
        "clippy",
        "test",
        "check_tests",
        "check_examples",
        "check_benches",
    ];
    let run_when = vec!["commit", "push"];

    let config_type = Select::new(
        "Do you want to make a global or local config?",
        config_types,
    )
    .prompt()?;

    let chosen_checks = MultiSelect::new("Select checks to run:", checks).prompt()?;

    let chosen_run_when = MultiSelect::new("Select when to run checks:", run_when).prompt()?;

    let cfg = PreflightConfig {
        run_when: chosen_run_when.into_iter().map(ToOwned::to_owned).collect(),
        checks: chosen_checks.into_iter().map(ToOwned::to_owned).collect(),
    };

    if config_type == "local" {
        let path = std::path::Path::new("./.preflight.toml");
        confy::store_path(path, cfg)?;
    } else {
        confy::store("cargo-preflight", "preflight", cfg)?;
    }

    Ok(())
}

fn preflight(matches: &clap::ArgMatches) -> Result<()> {
    let cfg = check_local_config()?;
    let init = matches.get_one::<bool>("init");
    let configure = matches.get_one::<bool>("config");
    if init == Some(&true) {
        println!("Initialising...");
        init_symlink(cfg)?;
    } else if configure == Some(&true) {
        update_config()?;
    } else {
        println!("{}", "Running Preflight Checks...".bold());
        preflight_checks(cfg)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();
    let binary_name = args.next().unwrap_or_default();

    let matches = if binary_name.ends_with("cargo") && args.next().as_deref() == Some("preflight") {
        cargo_subcommand(args)
    } else {
        standalone_command(args)
    };

    preflight(&matches)?;
    Ok(())
}
