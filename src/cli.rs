use anyhow::Result;
use colored::Colorize;
use inquire::{Confirm, MultiSelect, Select, Text};
use std::fs::exists;
use tabled::{
    Table,
    settings::{Reverse, Rotate, Style},
};

use crate::{
    autocomplete::{GlobalBranchCompleter, LocalBranchCompleter},
    config::{PreflightConfig, PreflightConfigWrapper, check_local_config},
    fix::{autofix, over_ride},
    preflight::preflight_checks,
};

const CLAP_STYLING: clap::builder::styling::Styles = clap::builder::styling::Styles::styled()
    .header(clap_cargo::style::HEADER)
    .usage(clap_cargo::style::USAGE)
    .literal(clap_cargo::style::LITERAL)
    .placeholder(clap_cargo::style::PLACEHOLDER)
    .error(clap_cargo::style::ERROR)
    .valid(clap_cargo::style::VALID)
    .invalid(clap_cargo::style::INVALID);

#[allow(clippy::cognitive_complexity)]
pub fn parse_args<I: Iterator<Item = String>>(args: I) -> clap::ArgMatches {
    let cmd = clap::Command::new("cargo-preflight")
        .styles(CLAP_STYLING)
        .arg(clap::arg!(--"init" "Initialise preflight in the current repository. This will add git hooks to run checks according to local/global config (priority in that order)").value_parser(clap::value_parser!(bool)))
        .arg(clap::arg!(--"ground" "Un-initialise preflight in the current repository. This will remove all git hooks").value_parser(clap::value_parser!(bool)))
        .arg(clap::Arg::new("REMOTE").hide(true))
        .arg(clap::arg!(--"config" "Configure preflight checks to run").value_parser(clap::value_parser!(bool)))
        .arg(clap::arg!(--"checklist" "Output the current configuration that will be applied in this repository").value_parser(clap::value_parser!(bool)));
    cmd.get_matches_from(args)
}

pub fn autofix_prompt(cfg: &PreflightConfig, index: usize) -> Result<()> {
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

pub fn update_config() -> Result<()> {
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
            "secrets",
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
            Text::new("Choose branches to run checks on (space seperated list):")
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

pub fn print_checklist() -> Result<()> {
    let cfg = check_local_config()?;
    let mut table = Table::new(cfg.preflight);
    table
        .with(Rotate::Right)
        .with(Reverse::columns(0))
        .with(Style::extended());
    let cfg_type = if exists("./.preflight.toml").expect("Can't check for local config") {
        "(Local)"
    } else {
        "(Global)"
    };
    println!(
        "{} {cfg_type}:",
        " 🛫 Current Active Preflight Checklist".bold()
    );
    println!("{table}");
    Ok(())
}
