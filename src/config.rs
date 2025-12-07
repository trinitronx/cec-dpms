//! Configuration module for CEC DPMS
//!
//! Provides configuration data structures, file I/O utilities, and path
//! resolution following the XDG Base Directory Specification.

pub mod cec_dpms_config;
pub mod loader;

pub use cec_dpms_config::{CecDpmsAdapterConfig, CecDpmsRootConfig};
pub use loader::{load_config, resolve_config_path};
