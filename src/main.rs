use anyhow::Result;
use colored::Colorize;
use serde_derive::{Deserialize, Serialize};
use std::{env, fs::exists, process::Command};
use thiserror::Error;

pub const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
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

#[derive(Error, Debug)]
pub enum PreflightError {
    #[error("{}{config}", "Invalid check in config: ".red())]
    InvalidCheck { config: String },

    #[error("{}{config}", "Invalid hook in config: ".red())]
    InvalidHook { config: String },

    #[error("\n    {}{fmt_output}", "[x] Formatting preflight check failed".red().bold())]
    FormatFailed { fmt_output: String },

    #[error("\n    {}{clippy_output}", "[x] Clippy preflight check failed:\n".red().bold())]
    ClippyFailed { clippy_output: String },

    #[error("\n    {}{check_outputs}", "[x] Check test preflight check failed:\n".red().bold())]
    CheckTestsFailed { check_outputs: String },

    #[error("\n    {}{check_outputs}", "[x] Check examples preflight check failed:\n".red().bold())]
    CheckExamplesFailed { check_outputs: String },

    #[error("\n    {}{check_outputs}", "[x] Check benches preflight check failed:\n".red().bold())]
    CheckBenchesFailed { check_outputs: String },

    #[error("\n    {}{test_outputs}", "[x] Test preflight check failed:\n".red().bold())]
    TestsFailed { test_outputs: String },
}

impl From<PreflightError> for std::io::Error {
    fn from(err: PreflightError) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{}", err))
    }
}

fn check_local_config() -> Result<PreflightConfig, confy::ConfyError> {
    if exists("./.cargo-preflight.toml").expect("Can't check for local config") {
        confy::load_path("./cargo-preflight.toml")
    } else {
        confy::load("cargo-preflight", None)
    }
}

fn preflight_checks(cfg: PreflightConfig) -> Result<()> {
    // get current workspace for tests and fmt
    for check in cfg.preflight_checks {
        match check.as_str() {
            "fmt" => cargo_fmt(),
            "clippy" => cargo_clippy(),
            "check_tests" => cargo_check_tests(),
            "check_examples" => cargo_check_examples(),
            "check_benches" => cargo_check_benches(),
            "test" => cargo_test(),
            _ => Err(PreflightError::InvalidCheck { config: check }.into()),
        }?
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
        .arg("--all-targets")
        .arg("--")
        .arg("-D warnings")
        .output()?;

    if output.status.success() {
        println!("    {}", "[√] Clippy preflight check passed".green());
        Ok(())
    } else {
        Err(PreflightError::ClippyFailed {
            clippy_output: String::from_utf8_lossy(&output.stdout).to_string(),
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
        }?
    }
    Ok(())
}

fn handle_cargo_subcommand<I: Iterator<Item = String>>(
    args: I,
) -> Result<clap::ArgMatches, Box<dyn std::error::Error>> {
    let cmd = clap::Command::new("cargo")
        .bin_name("cargo")
        .styles(CLAP_STYLING)
        .subcommand_required(true)
        .subcommand(
            clap::command!("preflight")
                .arg(clap::arg!(--"init").value_parser(clap::value_parser!(bool)))
                .arg(clap::arg!(<REMOTE>).value_parser(clap::value_parser!(String))),
        );
    let matches = cmd.get_matches_from(args);
    let matches = match matches.subcommand() {
        Some(("preflight", matches)) => matches,
        _ => unreachable!("clap should ensure we don't get here"),
    };
    Ok(matches.to_owned())
}

fn handle_standalone_command<I: Iterator<Item = String>>(
    args: I,
) -> Result<clap::ArgMatches, Box<dyn std::error::Error>> {
    let cmd = clap::Command::new("cargo-preflight")
        .styles(CLAP_STYLING)
        .arg(clap::arg!(--"init" "Initialise preflight in the current repository. This will add git hooks depending on local/global config (priority in that order)").value_parser(clap::value_parser!(bool)))
        .arg(clap::Arg::new("REMOTE").hide(true));
    Ok(cmd.get_matches_from(args))
}

fn update_config(matches: clap::ArgMatches) -> Result<()> {
    todo!();
}

fn preflight(matches: clap::ArgMatches) -> Result<()> {
    let init = matches.get_one::<bool>("init");
    let cfg = check_local_config()?;
    if let Some(true) = init {
        println!("{}", "Initialising...");
        init_symlink(cfg)?;
    } else {
        println!("{}", "Running Preflight Checks...".bold());
        preflight_checks(cfg)?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();
    let binary_name = args.next().unwrap_or_default();

    // Check if invoked as `cargo preflight`
    let handle_output =
        if binary_name.ends_with("cargo") && args.next().as_deref() == Some("preflight") {
            // Handle as a Cargo subcommand
            handle_cargo_subcommand(args)
        } else {
            // Handle as a standalone binary
            handle_standalone_command(args)
        };

    if let Ok(matches) = handle_output {
        preflight(matches)?;
    }
    Ok(())
}
