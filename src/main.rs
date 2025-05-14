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
//! ## Possible Options
//!
//! ```toml
//! [[preflight]] # Create new table entry for different behaviours
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
//!
//! ## Example Config:
//!
//! ```toml
//! [[preflight]]
//! run_when = ["commit"]
//! branches = []
//! checks = [
//!     "fmt",
//!     "clippy",
//! ]
//! autofix = true
//! over_ride = true
//!
//! [[preflight]]
//! run_when = ["push"]
//! branches = [
//!     "main",
//!     "master",
//! ]
//! checks = [
//!     "fmt",
//!     "clippy",
//!     "test",
//!     "unused_deps",
//! ]
//! autofix = false
//! over_ride = false
//! ```
//!
//! # Using Preflight
//!
//! Preflight can be enabled in a repository by running:
//!
//! ```sh
//! cargo preflight --init
//! ```
//!
//! _Note: Currently, Preflight only supports Linux systems._
//!

mod autocomplete;
mod checks;
mod cli;
mod config;
mod error;
mod fix;
mod git;
mod preflight;
mod util;

use std::env;

use crate::{cli::parse_args, preflight::preflight};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();
    let hook_arg = args.next().unwrap_or_default();
    let hook = hook_arg.split('-').next_back().unwrap_or_default();

    let matches = parse_args(args);

    preflight(&matches, hook)?;

    println!("TESTING SECRETS");
    checks::check_secrets()?;
    Ok(())
}
