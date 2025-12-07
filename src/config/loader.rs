//! Configuration file loading utilities
//!
//! Handles file path resolution and config file loading/parsing.
//! Follows XDG Base Directory Specification for config file location.

use super::cec_dpms_config::CecDpmsConfig;
use directories::ProjectDirs;
use simplelog::{paris, warn};
use std::path::Path;

/// Resolve the config file path following XDG Base Directory specification
///
/// Looks for config file in the following order:
/// 1. `$XDG_CONFIG_HOME/cec-dpms/config.yaml` (or `~/.config/cec-dpms/config.yaml` if `$XDG_CONFIG_HOME` not set)
/// 2. `/etc/cec-dpms/config.yaml` (system-wide fallback)
///
/// Returns the path to the first config file found, or the XDG default if none exist.
pub fn resolve_config_path() -> std::path::PathBuf {
    // Get XDG config directory for cec-dpms application
    let proj_dirs = ProjectDirs::from("", "", "cec-dpms");
    match &proj_dirs {
        Some(xdg) => {
            let user_config = xdg.config_dir().join("config.yaml");
            if user_config.exists() {
                return user_config;
            } else {
                warn!(
                    "user config: {:?} exists? {}",
                    user_config,
                    user_config.exists()
                );
            }
        }
        None => {
            warn!("user config could not be found in XDG BaseDirs");
        }
    }

    // Try system-wide config
    let system_config = Path::new("/etc/cec-dpms/config.yaml");
    if system_config.exists() {
        return system_config.to_path_buf();
    } else {
        warn!(
            "system config: {:?} exists? {}",
            system_config,
            system_config.exists()
        );
    }

    // Return XDG default even if it doesn't exist
    // (load will fail with a readable error message)
    if let Some(xdg) = proj_dirs {
        warn!(
            "Falling back to non-existent user config: {:?}",
            xdg.config_dir().join("config.yaml")
        );
        xdg.config_dir().join("config.yaml")
    } else {
        // Fallback if ProjectDirs creation fails
        // (internal project_dirs_from_path call returned None)
        warn!("Falling back to HOME or project user config");
        std::path::PathBuf::from(format!(
            "{}/.config/cec-dpms/config.yaml",
            std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
        ))
    }
}

/// Load and parse the configuration from a file
///
/// # Arguments
///
/// * `path` - Path to the configuration file (typically YAML format)
///
/// # Returns
///
/// Returns a `CecDpmsConfig` containing cec-dpms configuration,
/// or an error if file reading or parsing fails.
///
/// # Example
///
/// ```ignore
/// let config_path = resolve_config_path();
/// let config = load_config(config_path.to_str().unwrap_or(""))?;
/// let adapter = config.find_adapter("/dev/ttyACM0")?;
/// ```
pub fn load_config(path: &str) -> Result<CecDpmsConfig, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    Ok(serde_saphyr::from_str(contents.as_str())?)
}
