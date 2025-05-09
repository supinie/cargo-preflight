use anyhow::Result;
use colored::Colorize;

use crate::{
    checks::{check_branch_rules, run_checks},
    cli::{autofix_prompt, print_checklist, update_config},
    config::{PreflightConfig, check_local_config},
    error::failed_check_index,
    fix::over_ride,
    git::{delete_symlink, init_symlink},
};

pub fn preflight_checks(cfg: &PreflightConfig, start: usize) -> Result<()> {
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

pub fn preflight(matches: &clap::ArgMatches, hook: &str) -> Result<()> {
    let cfg = check_local_config()?;
    let init = matches.get_one::<bool>("init");
    let ground = matches.get_one::<bool>("ground");
    let configure = matches.get_one::<bool>("config");
    let checklist = matches.get_one::<bool>("checklist");
    if init == Some(&true) {
        println!("Initialising...");
        init_symlink()?;
    } else if ground == Some(&true) {
        println!("Closing hanger doors...");
        delete_symlink()?;
    } else if configure == Some(&true) {
        update_config()?;
    } else if checklist == Some(&true) {
        print_checklist()?;
    } else {
        println!("{}", "🛫 Running Preflight Checks...".bold());
        for config in &cfg.preflight {
            if config.run_when.contains(&hook.to_owned()) {
                preflight_checks(config, 0)?;
            } else if hook == "preflight" {
                println!("Running all defined preflight checks...");
                println!("{:?} checks:", config.run_when);
                preflight_checks(config, 0)?;
            }
        }
    }
    Ok(())
}
