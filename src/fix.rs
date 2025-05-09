use anyhow::Result;
use colored::Colorize;
use inquire::Confirm;
use std::process::Command;

use crate::{config::PreflightConfig, error::PreflightError, preflight::preflight_checks};

pub fn over_ride(cfg: &PreflightConfig, index: usize) -> Result<()> {
    let check = &cfg.checks[index];
    let ans = Confirm::new(&format!("Do you want to override {check} preflight check?",))
        .with_default(false)
        .with_help_message(&format!(
            "This will skip {check} and continue preflight checks",
        ))
        .prompt()?;

    if ans {
        println!("Skipping {check}...");
        preflight_checks(cfg, index + 1)?;
    } else {
        return Err(PreflightError::OverrideCancelled {
            failed_check: check.to_owned(),
        }
        .into());
    }

    Ok(())
}

pub fn fix_cargo_fmt() -> Result<()> {
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

pub fn fix_cargo_clippy() -> Result<()> {
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

pub fn autofix(check: &str) -> Result<()> {
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
