use anyhow::Result;
use serde_derive::{Deserialize, Serialize};
use std::{env, fs::exists};
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
    #[error("Invalid check in config: {0}")]
    InvalidCheck(String),

    #[error("Invalid hook in config: {0}")]
    InvalidHook(String),
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
            "test" => cargo_test(),
            _ => Err(PreflightError::InvalidCheck(check).into()),
        }?
    }
    Ok(())
}

fn cargo_fmt() -> Result<()> {
    println!("testing fmt!");
    Ok(())
}

fn cargo_test() -> Result<()> {
    println!("Running tests!");
    Ok(())
}

fn init_symlink(cfg: PreflightConfig) -> Result<()> {
    let mut path = dirs::home_dir().expect("No valid home dir found");
    path.push(".cargo/bin/cargo-preflight");
    for hook in cfg.run_when {
        match hook.as_str() {
            "commit" => std::os::unix::fs::symlink(&path, "./.git/hooks/pre-commit"),
            "push" => std::os::unix::fs::symlink(&path, "./.git/hooks/pre-push"),
            _ => Err(PreflightError::InvalidHook(hook).into()),
        }?
    }
    Ok(())
}

fn handle_cargo_subcommand<I: Iterator<Item = String>>(
    args: I,
) -> Result<clap::ArgMatches, Box<dyn std::error::Error>> {
    println!("subcommand");
    let cmd = clap::Command::new("cargo")
        .bin_name("cargo")
        .styles(CLAP_STYLING)
        .subcommand_required(true)
        .subcommand(
            clap::command!("preflight")
                .arg(clap::arg!(--"init").value_parser(clap::value_parser!(bool))),
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
    println!("standalone");
    let cmd = clap::Command::new("cargo-preflight")
        .styles(CLAP_STYLING)
        .arg(clap::arg!(--"init").value_parser(clap::value_parser!(bool)));
    Ok(cmd.get_matches_from(args))
}

fn preflight(matches: clap::ArgMatches) -> Result<()> {
    let init = matches.get_one::<bool>("init");
    let cfg = check_local_config()?;
    if let Some(true) = init {
        println!("Initialising...");
        init_symlink(cfg)?;
    } else {
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
