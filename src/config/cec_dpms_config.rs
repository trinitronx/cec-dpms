/// config: A config module supporting a YAML config file
use arrayvec::ArrayVec;
use cec_rs::CecLogicalAddress;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct CecDpmsConfig {
    pub hdmi_port: u8,
    base_device: String, // Deserialize as string, convert to enum
    pub activate_source: bool,
    pub physical_address: u16,
    pub device_types: ArrayVec<String, 5>,
}

/// CecDpmsConfig implementation
impl CecDpmsConfig {
    /// CecDpmsConfig file loader
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

    /// Convert `base_device` `String` into a `CecLogicalAddress` `enum`.
    ///
    /// This as a getter method to coerce a `String` provided in the config file
    /// into a usable `enum` variant.
    pub fn base_device(&self) -> CecLogicalAddress {
        match self.base_device.to_lowercase().as_str() {
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
        }
    }
}

/// Display trait implementation for CecDpmsConfig
///
/// Print the CecDpmsConfig contents as a single-line representation.
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
            "CecDpmsConfig {{ hdmi_port: {}, base_device: {}, activate_source: {}, physical_address: {} }}",
            self.hdmi_port, self.base_device, self.activate_source, hex_addr
        )
    }
}

/// Debug trait implementation for CecDpmsConfig
///
/// Print the CecDpmsConfig contents with more
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
            .field("base_device", &self.base_device)
            .field("activate_source", &self.activate_source)
            .field("physical_address", &hex_addr)
            .field("device_types", &self.device_types)
            .finish()
    }
}
