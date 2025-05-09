use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::exists;
use tabled::Tabled;

use crate::util::{display_checks, display_vecs};

#[derive(Debug, Serialize, Deserialize, Tabled)]
pub struct PreflightConfig {
    #[tabled(display = "display_vecs")]
    pub run_when: Vec<String>,
    #[tabled(display = "display_vecs")]
    pub branches: Vec<String>,
    #[tabled(display = "display_checks")]
    pub checks: Vec<String>,
    pub autofix: bool,
    #[tabled(rename = "override")]
    pub over_ride: bool,
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
pub struct PreflightConfigWrapper {
    pub preflight: Vec<PreflightConfig>,
}

impl Default for PreflightConfigWrapper {
    fn default() -> Self {
        Self {
            preflight: vec![PreflightConfig::default()],
        }
    }
}

pub fn check_local_config() -> Result<PreflightConfigWrapper, confy::ConfyError> {
    if exists("./.preflight.toml").expect("Can't check for local config") {
        confy::load_path("./.preflight.toml")
    } else {
        confy::load("cargo-preflight", "preflight")
    }
}
