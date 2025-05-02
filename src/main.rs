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
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! ## _Warning: Preflight is currently in development, and will be subject to breaking changes._
//!
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
//! `cargo install cargo-preflight`
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
//! _Remember to re-initialise in your repository if you change the `run_when` configuration, as the git hooks will need to be renewed._
//!
//! ## Possible Options
//!
//! ```toml
//! run_when = [
//!     "commit",
//!     "push",
//! ] # Default values: ["push"]
//!
//! # List of branch names to run on, below is an example.
//! # If the list is empty (default), then it will run on any branch.
//! branches = [
//!     "main",
//!     "my_feature",
//!     "supinie_dev",
//! ] # Default values: []
//!
//! checks = [
//!     "fmt", # `cargo fmt -- --check`
//!     "clippy", # `cargo clippy -- -D warnings`
//!     "test", # `cargo test`
//!     "check_tests", # `cargo check --tests`
//!     "check_examples",  # `cargo check --examples`
//!     "check_benches", # `cargo check --benches`
//!     "unused_deps", # uses `cargo-shear`
//! ] # Default values: ["fmt", "test"]
//!
//! autofix = false # Enables autofix functionality (for fmt and clippy)
//!
//! over_ride = false # Enables override functionality
//! ```
//! # Using Preflight
//!
//! Preflight can be enabled in a repository by running:
//!
//! ```sh
//! cargo preflight --init
//! ```
//!
//! Preflight can also be run as a one-off test with the `cargo preflight` command.
//!
//! _Note: Currently, Preflight only supports Linux systems._
//!

use anyhow::Result;
use cargo_shear::{CargoShear, cargo_shear_options};
use colored::Colorize;
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use git2::{BranchType, Repository};
use inquire::{
    Autocomplete, Confirm, CustomUserError, MultiSelect, Select, Text, autocompletion::Replacement,
};
use serde::{Deserialize, Serialize};
use std::{
    env,
    fs::exists,
    io::Read,
    process::{Command, ExitCode},
};
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
    branches: Vec<String>,
    checks: Vec<String>,
    autofix: bool,
    over_ride: bool,
}

impl Default for PreflightConfig {
    fn default() -> Self {
        Self {
            run_when: vec!["push".into()],
            branches: vec![],
            checks: vec!["fmt".into(), "test".into()],
            autofix: true,
            over_ride: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PreflightConfigWrapper {
    preflight: Vec<PreflightConfig>,
}

impl Default for PreflightConfigWrapper {
    fn default() -> Self {
        Self {
            preflight: vec![PreflightConfig::default()],
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

#[derive(Clone, Default)]
struct LocalBranchCompleter {
    input: String,
    branches: Vec<String>,
}

#[derive(Clone, Default)]
struct GlobalBranchCompleter {
    input: String,
    branches: Vec<String>,
}

trait BranchCompleter {
    fn update_input(&mut self, input: &str);
    fn get_branches(&self) -> &[String];

    fn fuzzy_sort(&self, input: &str) -> Vec<(String, i64)> {
        let mut matches: Vec<(String, i64)> = self
            .get_branches()
            .iter()
            .filter_map(|branch| {
                SkimMatcherV2::default()
                    .smart_case()
                    .fuzzy_match(branch, input)
                    .map(|score| (branch.clone(), score))
            })
            .collect();

        matches.sort_by(|a, b| b.1.cmp(&a.1));
        matches
    }

    fn get_last_word(input: &str) -> &str {
        if input.chars().nth(input.len() - 1) == Some(' ') {
            return "";
        }
        input.split_whitespace().last().unwrap_or("")
    }

    fn get_selected_branches(input: &str) -> Vec<String> {
        input.split_whitespace().map(String::from).collect()
    }
}

impl BranchCompleter for LocalBranchCompleter {
    fn update_input(&mut self, input: &str) {
        if input == self.input && !self.branches.is_empty() {
            return;
        }

        input.clone_into(&mut self.input);
        self.branches.clear();

        if let Ok(branches) = get_branches() {
            self.branches = branches;
        } else {
            self.branches = vec!["main".to_owned(), "master".to_owned()];
        }
    }

    fn get_branches(&self) -> &[String] {
        &self.branches
    }
}

impl BranchCompleter for GlobalBranchCompleter {
    fn update_input(&mut self, input: &str) {
        if input == self.input && !self.branches.is_empty() {
            return;
        }

        input.clone_into(&mut self.input);
        self.branches.clear();

        self.branches = vec!["main".to_owned(), "master".to_owned()];
    }

    fn get_branches(&self) -> &[String] {
        &self.branches
    }
}

macro_rules! impl_autocomplete {
    ($type:ty) => {
        impl Autocomplete for $type {
            fn get_suggestions(
                &mut self,
                input: &str,
            ) -> std::result::Result<Vec<String>, CustomUserError> {
                self.update_input(input);

                let last_word = Self::get_last_word(input);
                let selected_branches = Self::get_selected_branches(input);

                let matches = self.fuzzy_sort(last_word);
                Ok(matches
                    .into_iter()
                    .map(|(branch, _)| branch)
                    .filter(|branch| !selected_branches.contains(branch))
                    .take(15)
                    .collect())
            }

            fn get_completion(
                &mut self,
                input: &str,
                highlighted_suggestion: Option<String>,
            ) -> std::result::Result<Replacement, CustomUserError> {
                self.update_input(input);

                let mut selected_branches = Self::get_selected_branches(input);

                Ok(if let Some(suggestion) = highlighted_suggestion {
                    selected_branches.pop();
                    Replacement::Some(
                        selected_branches
                            .into_iter()
                            .chain(std::iter::once(suggestion))
                            .collect::<Vec<_>>()
                            .join(" "),
                    )
                } else {
                    let last_word = Self::get_last_word(input);
                    let matches = self.fuzzy_sort(last_word);

                    if let Some((branch, _)) = matches.first() {
                        selected_branches.pop();
                        Replacement::Some(
                            selected_branches
                                .into_iter()
                                .chain(std::iter::once(branch.clone()))
                                .collect::<Vec<_>>()
                                .join(" "),
                        )
                    } else {
                        Replacement::None
                    }
                })
            }
        }
    };
}

impl_autocomplete!(LocalBranchCompleter);
impl_autocomplete!(GlobalBranchCompleter);

fn check_local_config() -> Result<PreflightConfigWrapper, confy::ConfyError> {
    if exists("./.preflight.toml").expect("Can't check for local config") {
        confy::load_path("./.preflight.toml")
    } else {
        confy::load("cargo-preflight", "preflight")
    }
}

fn run_checks(checks: &[String]) -> Result<()> {
    for check in checks {
        match check.as_str() {
            "fmt" => cargo_fmt(),
            "clippy" => cargo_clippy(),
            "check_tests" => cargo_check_tests(),
            "check_examples" => cargo_check_examples(),
            "check_benches" => cargo_check_benches(),
            "test" => cargo_test(),
            "unused_deps" => shear(),
            _ => Err(PreflightError::InvalidCheck {
                config: check.to_owned(),
            }
            .into()),
        }?;
    }
    Ok(())
}

fn cargo_fmt() -> Result<()> {
    let output = Command::new("cargo")
        .args(["fmt", "--", "--check"])
        .output()?;

    if output.status.success() {
        println!("    {}", "[√] Formatting preflight check passed".green());
        Ok(())
    } else {
        println!("{}", String::from_utf8_lossy(&output.stderr));
        Err(PreflightError::FormatFailed {
            fmt_output: String::from_utf8_lossy(&output.stdout).to_string(),
        }
        .into())
    }
}

fn cargo_clippy() -> Result<()> {
    let output = Command::new("cargo")
        .args(["clippy", "--", "-D", "warnings"])
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
    let output = Command::new("cargo").args(["check", "--tests"]).output()?;

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
        .args(["check", "--examples"])
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
        .args(["check", "--benches"])
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

fn shear() -> Result<()> {
    let options = cargo_shear_options()
        .run_inner([env::current_dir()?.as_os_str()].as_slice())
        .map_err(|_| anyhow::anyhow!("Parse failure"))?;
    let mut buf = gag::BufferRedirect::stdout()?;
    let exit_code = CargoShear::new(options).run();
    let mut output = String::new();
    buf.read_to_string(&mut output)?;
    drop(buf);

    match exit_code {
        code if code == ExitCode::from(0) => {
            println!(
                "{}",
                "    [√] Unused dependencies preflight check passed".green()
            );
            Ok(())
        }
        code if code == ExitCode::from(1) => Err(PreflightError::ShearFailed {
            shear_output: output,
        }
        .into()),
        _ => Err(PreflightError::ShearFailed {
            shear_output: "Processing error during shear...".to_owned(),
        }
        .into()),
    }
}

fn init_symlink() -> Result<()> {
    let mut path = dirs::home_dir().expect("No valid home dir found");
    path.push(".cargo/bin/cargo-preflight");
    std::os::unix::fs::symlink(&path, "./.git/hooks/pre-commit")?;
    std::os::unix::fs::symlink(&path, "./.git/hooks/pre-push")?;
    Ok(())
}

fn get_current_branch_name() -> Option<String> {
    let repo = Repository::open(".").ok()?;
    let head = repo.head().ok()?;
    head.shorthand().map(String::from)
}

fn get_branches() -> Result<Vec<String>, git2::Error> {
    let repo = Repository::open(".")?;

    // Collect all branches into a vector
    let branches = repo
        .branches(Some(BranchType::Local))?
        .filter_map(|branch_result| match branch_result {
            Ok((branch, _)) => branch.name().ok().flatten().map(String::from),
            Err(_) => None, // Ignore branches that fail to load
        })
        .collect();

    Ok(branches)
}

fn parse_args<I: Iterator<Item = String>>(args: I) -> clap::ArgMatches {
    let cmd = clap::Command::new("cargo-preflight")
        .styles(CLAP_STYLING)
        .arg(clap::arg!(--"init" "Initialise preflight in the current repository. This will add git hooks depending on local/global config (priority in that order)").value_parser(clap::value_parser!(bool)))
        .arg(clap::Arg::new("REMOTE").hide(true))
        .arg(clap::arg!(--"config" "Configure preflight checks to run").value_parser(clap::value_parser!(bool)));
    cmd.get_matches_from(args)
}

fn update_config() -> Result<()> {
    let mut preflight_configs = Vec::new();

    let config_types = vec!["global", "local"];
    let config_type = Select::new(
        "Do you want to make a global or local config?",
        config_types,
    )
    .with_vim_mode(true)
    .prompt()?;

    loop {
        let checks = vec![
            "fmt",
            "clippy",
            "test",
            "unused_deps",
            "check_tests",
            "check_examples",
            "check_benches",
        ];
        let run_when = vec!["commit", "push"];

        let chosen_checks = MultiSelect::new("Select checks to run:", checks)
            .with_vim_mode(true)
            .prompt()?;

        let chosen_run_when = MultiSelect::new("Select when to run checks:", run_when)
            .with_vim_mode(true)
            .prompt()?;

        let branches = if config_type == "global" {
            Text::new("Choose branches to run checks on:")
                .with_autocomplete(GlobalBranchCompleter::default())
                .with_help_message("Leave blank to run on any branch")
                .prompt()
        } else {
            Text::new("Choose branches to run checks on (space separated list):")
                .with_autocomplete(LocalBranchCompleter::default())
                .with_help_message("Leave blank to run on any branch")
                .prompt()
        }?;

        let autofix = Confirm::new("Enable autofix functionality?")
            .with_default(false)
            .with_help_message(
                "Where possible, this will enable you to automatically apply suggestions",
            )
            .prompt()?;

        let over_ride = Confirm::new("Enable override functionality?")
            .with_default(false)
            .with_help_message("This will allow you to override Preflight on failed checks")
            .prompt()?;

        let cfg = PreflightConfig {
            run_when: chosen_run_when.into_iter().map(ToOwned::to_owned).collect(),
            branches: branches.split_whitespace().map(ToOwned::to_owned).collect(),
            checks: chosen_checks.into_iter().map(ToOwned::to_owned).collect(),
            autofix,
            over_ride,
        };

        preflight_configs.push(cfg);

        let add_more = Confirm::new("Do you want to add another configuration?")
            .with_default(false)
            .with_help_message("Choose 'yes' to create another configuration.")
            .prompt()?;

        if !add_more {
            break;
        }
    }

    let wrapped_configs = PreflightConfigWrapper {
        preflight: preflight_configs,
    };

    // Save the configurations
    if config_type == "global" {
        confy::store("cargo-preflight", "preflight", wrapped_configs)?;
    } else {
        let path = std::path::Path::new("./.preflight.toml");
        confy::store_path(path, wrapped_configs)?;
    }

    Ok(())
}

fn failed_check_index(checks: &[String], error: &PreflightError) -> Option<usize> {
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

fn over_ride(cfg: &PreflightConfig, index: usize) -> Result<()> {
    let check = &cfg.checks[index];
    let ans = Confirm::new(&format!(
        "Do you want to override {} preflight check?",
        check
    ))
    .with_default(false)
    .with_help_message(&format!(
        "This will skip {} and continue preflight checks",
        check
    ))
    .prompt()?;

    if ans {
        println!("Skipping {}...", check);
        preflight_checks(cfg, index + 1)?;
    } else {
        return Err(PreflightError::OverrideCancelled {
            failed_check: check.to_owned(),
        }
        .into());
    }

    Ok(())
}

fn fix_cargo_fmt() -> Result<()> {
    let output = Command::new("cargo").arg("fmt").output()?;

    if output.status.success() {
        println!("    {}", "[√] Applying fmt successful".yellow());
        Ok(())
    } else {
        println!("{}", String::from_utf8_lossy(&output.stderr));
        Err(PreflightError::FormatFailed {
            fmt_output: String::from_utf8_lossy(&output.stdout).to_string(),
        }
        .into())
    }
}

fn fix_cargo_clippy() -> Result<()> {
    let output = Command::new("cargo")
        .env("__CARGO_FIX_YOLO", "1")
        .args(["clippy", "--fix", "--allow-dirty"])
        .output()?;

    if output.status.success() {
        println!(
            "    {}",
            "[√] Applying clippy suggestions successful".yellow()
        );
        Ok(())
    } else {
        Err(PreflightError::ClippyFailed {
            clippy_output: String::from_utf8_lossy(&output.stderr).to_string(),
        }
        .into())
    }
}

fn autofix(check: &str) -> Result<()> {
    match check {
        "fmt" => fix_cargo_fmt(),
        "clippy" => fix_cargo_clippy(),
        _ => Err(PreflightError::InvalidCheck {
            config: check.to_owned(),
        }
        .into()),
    }?;
    Ok(())
}

fn autofix_prompt(cfg: &PreflightConfig, index: usize) -> Result<()> {
    let ans = Confirm::new(&format!(
        "Do you want to automatically apply {} suggestions?",
        &cfg.checks[index]
    ))
    .with_default(false)
    .with_help_message(
        "WARNING: This will apply changes to your dirty workspace, and may be potentially destructive.\nNo will end and fail preflight checks, yes will apply suggestions and continue",
    )
    .prompt();

    match ans {
        Ok(false) => {
            if cfg.over_ride {
                over_ride(cfg, index)
            } else {
                Ok(())
            }
        }
        Ok(true) => {
            autofix(&cfg.checks[index])?;
            preflight_checks(cfg, index)
        }
        Err(_) => {
            println!("Error autofixing preflight");
            Ok(())
        }
    }
}

fn check_branch_rules(branches: &[String]) -> bool {
    if branches.is_empty() {
        return true;
    }
    get_current_branch_name().map_or_else(|| {
        println!(
            "{}",
            "It looks like you're not on a git branch... Preflight will continue, but there may be an error later".italic()
        );
        true
    }, |branch| branches.contains(&branch))
}

fn preflight_checks(cfg: &PreflightConfig, start: usize) -> Result<()> {
    if !check_branch_rules(&cfg.branches) {
        println!("Branch not included in preflight checks, exiting...");
        return Ok(());
    }
    let stopped_at = match run_checks(&cfg.checks[start..]) {
        Ok(()) => None,
        Err(e) => {
            println!("{e:?}");
            e.downcast_ref()
                .and_then(|preflight_err| failed_check_index(&cfg.checks, preflight_err))
        }
    };

    if let (true, Some(index)) = (cfg.autofix, stopped_at) {
        autofix_prompt(cfg, index)?;
    } else if let (true, Some(index)) = (cfg.over_ride, stopped_at) {
        over_ride(cfg, index)?;
    }

    Ok(())
}

fn preflight(matches: &clap::ArgMatches, hook: &str) -> Result<()> {
    let cfg = check_local_config()?;
    let init = matches.get_one::<bool>("init");
    let configure = matches.get_one::<bool>("config");
    if init == Some(&true) {
        println!("Initialising...");
        init_symlink()?;
    } else if configure == Some(&true) {
        update_config()?;
    } else {
        println!("{}", "🛫 Running Preflight Checks...".bold());
        for config in &cfg.preflight {
            if config.run_when.contains(&hook.to_owned()) {
                preflight_checks(config, 0)?;
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();
    let hook_arg = args.next().unwrap_or_default();
    let hook = hook_arg.split('-').last().unwrap_or_default();

    let matches = parse_args(args);

    preflight(&matches, hook)?;
    Ok(())
}
