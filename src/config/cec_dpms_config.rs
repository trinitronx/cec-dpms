/// config: A config module supporting a YAML config file
use arrayvec::ArrayVec;
use cec_rs::{CecDeviceType, CecLogicalAddress};
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

mod cec_device_types_serde {
    use arrayvec::ArrayVec;
    use cec_rs::CecDeviceType;
    use serde::{de, Deserializer, Serializer};
    use std::fmt;

    /// Serialize `device_types` `ArrayVec<CecDeviceType, 5>` into a sequence of
    /// strings.
    ///
    /// This is a `Serialize` trait implementation to convert an array of
    /// `CecDeviceType` enum variants into strings for use in config file and
    /// serialization formats.
    pub fn serialize<S>(
        devices: &ArrayVec<CecDeviceType, 5>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(devices.len()))?;
        for device in devices {
            seq.serialize_element(&format!("{:?}", device))?;
        }
        seq.end()
    }

    /// Deserialize and convert a sequence of device type strings into
    /// `ArrayVec<CecDeviceType, 5>`.
    ///
    /// This is a `Deserialize` trait implementation to coerce an `Array` of
    /// strings provided in the config file into an `ArrayVec` of
    /// `CecDeviceType` enum variants. Max capacity `5` of the `ArrayVec` is set
    /// to be compatible with the `cec-rs::CecDeviceTypeVec` destination type.
    /// Matching is case-insensitive. Unknown values are silently skipped.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<ArrayVec<CecDeviceType, 5>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DeviceTypesVisitor;

        impl<'de> de::Visitor<'de> for DeviceTypesVisitor {
            type Value = ArrayVec<CecDeviceType, 5>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a sequence of device type strings")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut devices = ArrayVec::new();

                while let Some(device_str) = seq.next_element::<String>()? {
                    match device_str.to_lowercase().as_str() {
                        "tv" => devices.push(CecDeviceType::Tv),
                        "recordingdevice" => devices.push(CecDeviceType::RecordingDevice),
                        "reserved" => devices.push(CecDeviceType::Reserved),
                        "tuner" => devices.push(CecDeviceType::Tuner),
                        "playbackdevice" => devices.push(CecDeviceType::PlaybackDevice),
                        "audiosystem" => devices.push(CecDeviceType::AudioSystem),
                        unknown => {
                            // Log unknown but don't fail - silently skip
                            eprintln!("Warning: Unknown device type in config: {}", unknown);
                        }
                    }

                    // Stop if we've reached capacity
                    if devices.is_full() {
                        break;
                    }
                }

                Ok(devices)
            }
        }

        deserializer.deserialize_seq(DeviceTypesVisitor)
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
    #[serde(with = "cec_device_types_serde")]
    pub device_types: ArrayVec<CecDeviceType, 5>,
}

/// `Default` trait implementation for `CecDpmsConfig`
///
/// Defaults to:
///
///     CecDpmsConfig {
///         hdmi_port: 1,
///         base_device: Tv,
///         activate_source: true,
///         physical_address: 0x1000,
///         device_types: [
///             PlaybackDevice,
///         ],
///     }
impl Default for CecDpmsConfig {
    fn default() -> Self {
        CecDpmsConfig {
            hdmi_port: CEC_DEFAULT_HDMI_PORT as u8,
            base_device: CecLogicalAddress::from_repr(CEC_DEFAULT_BASE_DEVICE as i32)
                .unwrap_or(CecLogicalAddress::Unknown),
            activate_source: false,
            physical_address: CEC_DEFAULT_PHYSICAL_ADDRESS.try_into().unwrap(),
            device_types: [CecDeviceType::PlaybackDevice].into_iter().collect(),
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
///
/// Again the hexidecimal CEC physical address is printed as a dot-separated
/// string.
///
/// **Note:** The internal representation is a `u16`, but the common dot-sparated CEC
/// address notation is used for `Debug` and pretty-print.
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
        let yaml = "hdmi_port: 4\n\
                          base_device: \"Tv\"\n\
                          activate_source: true\n\
                          physical_address: 0x3000\n\
                          device_types:\n\
                          - RecordingDevice\n\
                          - PlaybackDevice";
        let expected = CecDpmsConfig {
            hdmi_port: 4,
            base_device: CecLogicalAddress::Tv,
            activate_source: true,
            physical_address: 0x3000,
            device_types: [
                CecDeviceType::RecordingDevice,
                CecDeviceType::PlaybackDevice,
            ]
            .into_iter()
            .collect(),
        };
        let parsed: CecDpmsConfig = serde_saphyr::from_str(yaml).unwrap();
        // Assert parsed config matches expected
        assert_eq!(parsed, expected);
    }

    #[test]
    fn test_cec_dpms_config_base_device_deserialize_case_insensitive() {
        let test_cases = vec![
            ("tv", CecLogicalAddress::Tv),
            ("TV", CecLogicalAddress::Tv),
            ("PlaybackDevice1", CecLogicalAddress::Playbackdevice1),
            ("RECORDINGDEVICE2", CecLogicalAddress::Recordingdevice2),
            ("audiosystem", CecLogicalAddress::Audiosystem),
            ("audiosystem", CecLogicalAddress::Audiosystem),
            ("InvalidDevice", CecLogicalAddress::Unknown),
            ("xyz123", CecLogicalAddress::Unknown),
        ];

        for (input, expected) in test_cases {
            let yaml = format!("base_device: \"{}\"", input);
            let config: CecDpmsConfig = serde_saphyr::from_str(&yaml).unwrap();
            assert_eq!(config.base_device, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_cec_dpms_config_base_device_deserialize() {
        let test_cases = vec![
            ("tv", CecLogicalAddress::Tv),
            ("recordingdevice1", CecLogicalAddress::Recordingdevice1),
            ("recordingdevice2", CecLogicalAddress::Recordingdevice2),
            ("tuner1", CecLogicalAddress::Tuner1),
            ("playbackdevice1", CecLogicalAddress::Playbackdevice1),
            ("audiosystem", CecLogicalAddress::Audiosystem),
            ("tuner2", CecLogicalAddress::Tuner2),
            ("tuner3", CecLogicalAddress::Tuner3),
            ("playbackdevice2", CecLogicalAddress::Playbackdevice2),
            ("recordingdevice3", CecLogicalAddress::Recordingdevice3),
            ("tuner4", CecLogicalAddress::Tuner4),
            ("playbackdevice3", CecLogicalAddress::Playbackdevice3),
            ("reserved1", CecLogicalAddress::Reserved1),
            ("reserved2", CecLogicalAddress::Reserved2),
            ("freeuse", CecLogicalAddress::Freeuse),
            ("unregistered", CecLogicalAddress::Unregistered),
            ("invalid", CecLogicalAddress::Unknown),
        ];

        for (input, expected) in test_cases {
            let yaml = format!("base_device: \"{}\"", input);
            let config: CecDpmsConfig = serde_saphyr::from_str(&yaml).unwrap();
            assert_eq!(config.base_device, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_cec_dpms_config_load_error() {
        let yaml = "---\n\
                          invalid_config: true";
        let expected = CecDpmsConfig::default();
        let parsed: CecDpmsConfig = serde_saphyr::from_str(yaml).unwrap();
        // Assert parsed config matches expected
        assert_eq!(parsed, expected);
    }
}
