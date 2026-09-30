//! Configuration module for CEC DPMS
//!
//! Provides configuration data structures, file I/O utilities, and path
//! resolution following the XDG Base Directory Specification.

mod cec_dpms_config;
mod loader;

// Make available to other modules in same crate, but not external crates
pub(crate) use cec_dpms_config::{CecDpmsAdapterConfig, CecDpmsRootConfig};
pub(crate) use loader::{load_config, resolve_config_path};
