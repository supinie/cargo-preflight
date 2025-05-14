use anyhow::Result;
use cargo_shear::{CargoShear, cargo_shear_options};
use colored::Colorize;
use ripsecrets::find_secrets;
use std::{
    env,
    io::Read,
    path::PathBuf,
    process::{Command, ExitCode},
};
use termcolor::{BufferWriter, ColorChoice};

use crate::{error::PreflightError, git::get_current_branch_name};

pub fn run_checks(checks: &[String]) -> Result<()> {
    for check in checks {
        match check.as_str() {
            "fmt" => cargo_fmt(),
            "clippy" => cargo_clippy(),
            "check_tests" => cargo_check_tests(),
            "check_examples" => cargo_check_examples(),
            "check_benches" => cargo_check_benches(),
            "test" => cargo_test(),
            "unused_deps" => shear(),
            "secrets" => secrets(),
            _ => Err(PreflightError::InvalidCheck {
                config: check.to_owned(),
            }
            .into()),
        }?;
    }
    Ok(())
}

pub fn cargo_fmt() -> Result<()> {
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

pub fn cargo_clippy() -> Result<()> {
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

pub fn cargo_check_tests() -> Result<()> {
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

pub fn cargo_check_examples() -> Result<()> {
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

pub fn cargo_check_benches() -> Result<()> {
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

pub fn cargo_test() -> Result<()> {
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

pub fn shear() -> Result<()> {
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

pub fn check_branch_rules(branches: &[String]) -> bool {
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

pub fn secrets() -> Result<()> {
    let mut buf = gag::BufferRedirect::stdout()?;
    let mut output = String::new();

    let ret = find_secrets(
        &[PathBuf::from(".")],
        &[],
        false,
        false,
        BufferWriter::stdout(ColorChoice::Never),
    );

    buf.read_to_string(&mut output)?;
    drop(buf);

    match ret {
        Ok(0) => Ok(()),
        Ok(num) => Err(PreflightError::SecretsFailed {
            ripsecrets_output: format!("Found {num} secret(s): \n{output}"),
        }
        .into()),
        Err(err) => Err(PreflightError::SecretsFailed {
            ripsecrets_output: err.to_string(),
        }
        .into()),
    }
}
