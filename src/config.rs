//! `CecDpmsConfig` struct supporting YAML config files

pub mod cec_dpms_config;
pub use cec_dpms_config::{CecDpmsConfig};
pub mod loader;

pub use loader::{load_config, resolve_config_path};
