/// config: A config module supporting a YAML config file
use arrayvec::ArrayVec;
use cec_rs::CecLogicalAddress;
use libcec_sys::{CEC_DEFAULT_BASE_DEVICE, CEC_DEFAULT_HDMI_PORT, CEC_DEFAULT_PHYSICAL_ADDRESS};
use serde::{Deserialize, Serialize};

mod cec_logical_address_serde {
    use cec_rs::CecLogicalAddress;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Serialize `base_device` `CecLogicalAddress` into a `String`.
    ///
    /// This is a `Serialize` trait implementation to convert a
    /// `CecLogicalAddress` `enum` variant into a `String` for use in config
    /// file and serialization formats.
    pub fn serialize<S>(addr: &CecLogicalAddress, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{:?}", addr))
    }

    /// Deserialize and convert `base_device` `String` into a
    /// `CecLogicalAddress` `enum`.
    ///
    /// This is a `Deserialize` trait implementation to coerce a `String`
    /// provided in the config file into a usable `enum` variant.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<CecLogicalAddress, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "tv" => CecLogicalAddress::Tv,
            "recordingdevice1" => CecLogicalAddress::Recordingdevice1,
            "recordingdevice2" => CecLogicalAddress::Recordingdevice2,
            "tuner1" => CecLogicalAddress::Tuner1,
            "playbackdevice1" => CecLogicalAddress::Playbackdevice1,
            "audiosystem" => CecLogicalAddress::Audiosystem,
            "tuner2" => CecLogicalAddress::Tuner2,
            "tuner3" => CecLogicalAddress::Tuner3,
            "playbackdevice2" => CecLogicalAddress::Playbackdevice2,
            "recordingdevice3" => CecLogicalAddress::Recordingdevice3,
            "tuner4" => CecLogicalAddress::Tuner4,
            "playbackdevice3" => CecLogicalAddress::Playbackdevice3,
            "reserved1" => CecLogicalAddress::Reserved1,
            "reserved2" => CecLogicalAddress::Reserved2,
            "freeuse" => CecLogicalAddress::Freeuse,
            "unregistered" => CecLogicalAddress::Unregistered,
            _ => CecLogicalAddress::Unknown,
        })
    }
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "snake_case")]
pub struct CecDpmsConfig {
    pub hdmi_port: u8,
    #[serde(with = "cec_logical_address_serde")]
    pub base_device: CecLogicalAddress,
    pub activate_source: bool,
    pub physical_address: u16,
    pub device_types: ArrayVec<String, 5>,
}

/// `Default` trait implementation for `CecDpmsConfig`
///
/// Defaults to:
///
///     CecDpmsConfig {
///            hdmi_port: 1,
///            base_device: "Tv",
///            activate_source: true,
///            physical_address: 0x1000,
///            device_types: [
///                "PlaybackDevice",
///            ],
///        }
impl Default for CecDpmsConfig {
    fn default() -> Self {
        CecDpmsConfig {
            hdmi_port: CEC_DEFAULT_HDMI_PORT as u8,
            base_device: CecLogicalAddress::from_repr(CEC_DEFAULT_BASE_DEVICE as i32)
                .unwrap_or(CecLogicalAddress::Unknown),
            activate_source: false,
            physical_address: CEC_DEFAULT_PHYSICAL_ADDRESS.try_into().unwrap(),
            device_types: ["PlaybackDevice"].into_iter().map(String::from).collect(),
        }
    }
}

/// `CecDpmsConfig` implementation
impl CecDpmsConfig {
    /// `CecDpmsConfig` file loader
    ///
    /// # Example Config
    ///
    ///     hdmi_port: 4
    ///     base_device: "Tv"
    ///     activate_source: true
    ///     physical_address: 0x3000
    ///     device_types:
    ///     - RecordingDevice
    ///     - PlaybackDevice
    ///
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_saphyr::from_str(contents.as_str())?)
    }
}

/// `Display` trait implementation for `CecDpmsConfig`
///
/// Print the `CecDpmsConfig` contents as a single-line representation.
///
/// Notably, print the hexidecimal CEC physical address as a dot-separated string.
impl std::fmt::Display for CecDpmsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let addr = self.physical_address;
        let hex_addr = format!(
            "{:x}.{:x}.{:x}.{:x}",
            (addr >> 12) & 0xF,
            (addr >> 8) & 0xF,
            (addr >> 4) & 0xF,
            addr & 0xF
        );

        write!(
            f,
            "CecDpmsConfig {{ hdmi_port: {}, base_device: {:?}, activate_source: {}, physical_address: {} }}",
            self.hdmi_port, self.base_device, self.activate_source, hex_addr
        )
    }
}

/// `Debug` trait implementation for `CecDpmsConfig`
///
/// Print the `CecDpmsConfig` contents with more detailed representation of the
/// internal struct contents.
impl std::fmt::Debug for CecDpmsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let addr = self.physical_address;
        let hex_addr = format!(
            "{:x}.{:x}.{:x}.{:x}",
            (addr >> 12) & 0xF,
            (addr >> 8) & 0xF,
            (addr >> 4) & 0xF,
            addr & 0xF
        );

        f.debug_struct("CecDpmsConfig")
            .field("hdmi_port", &self.hdmi_port)
            .field("base_device", &format!("{:?}", self.base_device))
            .field("activate_source", &self.activate_source)
            .field("physical_address", &hex_addr)
            .field("device_types", &self.device_types)
            .finish()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_cec_dpms_config_load() {
        let yaml = r#"
hdmi_port: 4
base_device: "Tv"
activate_source: true
physical_address: 0x3000
device_types:
- RecordingDevice
- PlaybackDevice
"#;
        let expected = CecDpmsConfig {
            hdmi_port: 4,
            base_device: CecLogicalAddress::Tv,
            activate_source: true,
            physical_address: 0x3000,
            device_types: ["RecordingDevice", "PlaybackDevice"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };
        let parsed: CecDpmsConfig = serde_saphyr::from_str(yaml).unwrap();
        // Assert parsed config matches expected
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_cec_dpms_config_base_device() {
        let cec_dpms_config = CecDpmsConfig {
            hdmi_port: 4,
            base_device: CecLogicalAddress::Tv,
            activate_source: true,
            physical_address: 0x3000,
            device_types: ["RecordingDevice", "PlaybackDevice"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };
        assert_eq!(cec_dpms_config.base_device, CecLogicalAddress::Tv);
    }

    #[test]
    fn test_cec_dpms_config_load_error() {
        let yaml = r#"
---
invalid_config: true
"#;
        let expected = CecDpmsConfig::default();
        let parsed: CecDpmsConfig = serde_saphyr::from_str(yaml).unwrap();
        // Assert parsed config matches expected
        assert_eq!(parsed, expected);
    }
}
