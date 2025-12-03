use clap::Parser;
use hostname;
use signal_hook::{consts::SIGINT, consts::SIGTERM, consts::SIGUSR1, consts::SIGUSR2};
use simplelog::*;
use std::cell::RefCell;
use std::error::Error;
use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{thread, time};

use arrayvec::ArrayVec;
extern crate cec_rs;
use cec_rs::{
    CecCommand, CecConnection, CecConnectionCfgBuilder, CecDatapacket, CecDeviceType,
    CecDeviceTypeVec, CecLogMessage, CecLogicalAddress, CecOpcode,
};
use libcec_sys::CEC_INVALID_PHYSICAL_ADDRESS;

use std::sync::atomic::AtomicUsize;

static GLOBAL_THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    /// Enable debug info
    #[clap(short, long)]
    debug: bool,

    /// input device path/name of CEC device
    #[clap(short, long, parse(from_os_str))]
    input: Option<std::path::PathBuf>,
}

fn logging_init(debug: bool) {
    let conf = ConfigBuilder::new()
        .set_time_format("%F, %H:%M:%S%.3f".to_string())
        .set_write_log_enable_colors(true)
        .build();

    let mut loggers = vec![];

    let console_logger: Box<dyn SharedLogger> = TermLogger::new(
        if debug {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        },
        conf.clone(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    );
    loggers.push(console_logger);

    CombinedLogger::init(loggers).expect("Cannot initialize logging subsystem");
}

fn on_command_received(command: CecCommand) {
    debug!(
        "onCommandReceived: opcode: {:?}, initiator: {:?}",
        command.opcode, command.initiator
    );
    // Note that Relaxed ordering doesn't synchronize anything
    // except the global thread counter itself.
    let old_thread_count = GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
    // Note that this number may not be true at the moment of printing
    // because some other thread may have changed static value already.
    debug!("live threads: {}", old_thread_count + 1);

    THREAD_CONNECTION.with(|connection| {
        debug!(
            "onCommandReceived: opcode type: {:?}",
            std::any::type_name_of_val(&command.opcode)
        );
        debug!("onCommandReceived: try to borrow the connection: {:?}", std::any::type_name_of_val(&connection));
        // Use the static CONNECTION variable instead of the thread-local one
        if let Some(Some(conn)) = CONNECTION.get() {
            debug!(
                "onCommandReceived: Connection successfully borrowed from thread-local storage: {:?}",
                std::any::type_name_of_val(&conn)
            );
            match command.opcode {
                CecOpcode::GiveDevicePowerStatus => {
                    debug!(
                        "onCommandReceived: Got a GiveDevicePowerStatus command!!: opcode: {:?}, initiator: {:?}, destination: {:?}, ack: {:?}, eom: {:?}, parameters: {:?}, opcode_set?: {:?}, transmit_timeout: {:?}",
                        command.opcode, command.initiator, command.destination, command.ack, command.eom, command.parameters, command.opcode_set, command.transmit_timeout
                    );

                    let mut a = ArrayVec::new();
                    a.push(0x00); // CEC_POWER_STATUS_ON
                    let packet = CecDatapacket(a);

                    let _ = conn.transmit(CecCommand {
                        initiator: CecLogicalAddress::Playbackdevice1,
                        destination: command.initiator,
                        opcode: CecOpcode::ReportPowerStatus,
                        parameters: packet,
                        eom: true,
                        ack: false,
                        opcode_set: false,
                        transmit_timeout: time::Duration::from_secs(1),
                    });
                }
                CecOpcode::ReportPowerStatus => {
                    debug!(
                        "onCommandReceived: Got a ReportPowerStatus command!!: opcode: {:?}, initiator: {:?}, destination: {:?}, ack: {:?}, eom: {:?}, parameters: {:?}, opcode_set?: {:?}, transmit_timeout: {:?}",
                        command.opcode, command.initiator, command.destination, command.ack, command.eom, command.parameters, command.opcode_set, command.transmit_timeout
                    );
                }
                CecOpcode::GivePhysicalAddress => {
                    debug!(
                        "onCommandReceived: Got a GivePhysicalAddress command!!: opcode: {:?}, initiator: {:?}, destination: {:?}, ack: {:?}, eom: {:?}, parameters: {:?}, opcode_set?: {:?}, transmit_timeout: {:?}",
                        command.opcode, command.initiator, command.destination, command.ack, command.eom, command.parameters, command.opcode_set, command.transmit_timeout
                    );
                    report_physical_address(&command, &conn);
              }
              CecOpcode::VendorCommand => {
                    debug!(
                        "onCommandReceived: Got a VendorCommand command!!: opcode: {:?}, initiator: {:?}, destination: {:?}, ack: {:?}, eom: {:?}, parameters: {:?}, opcode_set?: {:?}, transmit_timeout: {:?}",
                        command.opcode, command.initiator, command.destination, command.ack, command.eom, command.parameters, command.opcode_set, command.transmit_timeout
                    );
                    handle_vendor_command(&command, &conn);
              }
              CecOpcode::DeviceVendorId => {
                    debug!(
                        "onCommandReceived: Got a DeviceVendorId command!!: opcode: {:?}, initiator: {:?}, destination: {:?}, ack: {:?}, eom: {:?}, parameters: {:?}, opcode_set?: {:?}, transmit_timeout: {:?}",
                        command.opcode, command.initiator, command.destination, command.ack, command.eom, command.parameters, command.opcode_set, command.transmit_timeout
                    );
                    handle_device_vendor_id(&command);
                }
                _ => {
                    debug!(
                        "onCommandReceived: Unknown command: opcode: {:?}, initiator: {:?}, destination: {:?}",
                        command.opcode, command.initiator, command.destination
                    );
                }
            }
        }
        else {
            debug!("<b><red>Error:</> Could not borrow the connection: {:?}", std::any::type_name_of_val(&connection));

            debug!("<b><red>Debug:</> RefCell wrapper type: {}", std::any::type_name_of_val(&connection));
            // Get the contents of RefCell
            let borrowed = connection.borrow();
            debug!("<b><red>Debug:</> After borrow() is_some()??: {:#?} (type: {})",
                borrowed.is_some(),
                std::any::type_name_of_val(&borrowed)
            );

            // Look at the Option value inside
            match *borrowed {
                Some(ref cec_conn) => {
                    debug!("Connection exists (type: {}) with:", std::any::type_name_of_val(cec_conn));
                    debug!("  - Logical addresses: {:?}", cec_conn.get_logical_addresses());
                    debug!("  - Active source: {:?}", cec_conn.get_active_source());
                    let foo = cec_conn.is_active_source(CecLogicalAddress::Playbackdevice1);
                    debug!("  - Is Playbackdevice1 active source?: {:#?}", foo);
                },
                None => {
                    debug!("Connection is None!");
                }
            }
    }
    GLOBAL_THREAD_COUNT.fetch_sub(1, Ordering::Relaxed);
})
}

fn on_log_message(log_message: CecLogMessage) {
    let log_prefix = "<black>libcec:</>";
    match log_message.level {
        cec_rs::CecLogLevel::All => trace!("{} {}", log_prefix, log_message.message),
        cec_rs::CecLogLevel::Debug | cec_rs::CecLogLevel::Traffic => {
            debug!("{} {}", log_prefix, log_message.message)
        }
        cec_rs::CecLogLevel::Notice => info!("{} {}", log_prefix, log_message.message),
        cec_rs::CecLogLevel::Warning => warn!("{} {}", log_prefix, log_message.message),
        cec_rs::CecLogLevel::Error => error!("{} {}", log_prefix, log_message.message),
    }
}

fn report_physical_address(command: &CecCommand, connection: &CecConnection) {
    // CONNECTION.with(|connection| {
    // if let Some(connection) = connection.borrow().as_ref() {

    // let phys_addr_result = CONNECTION_CONFIG.with(|cfg| {
    //     cfg.borrow()
    //         .as_ref()
    //         .ok_or("No connection config available")
    //         .as_deref()
    //         .map_or(Err("No physical address found in connection config"), |c| {
    //             Ok(c.physical_address)
    //         })
    // });
    let phys_addr_result = connection.0.physical_address;
    match phys_addr_result {
        // Ok(Some(addr)) => {
        Some(addr) => {
            // let addr = cfg.physical_address.clone();
            info!("Reporting Physical address: {:#06x}", addr);
            // let pkt_data: ArrayVec<u8, 64> = ArrayVec::from(addr.to_be_bytes().into());
            let mut pkt_data: ArrayVec<u8, 64> = addr.to_be_bytes().iter().copied().collect();
            pkt_data.push(
                get_logical_address_or_default(Some(CecLogicalAddress::Playbackdevice1)).repr()
                    as u8,
            );
            // let mut raw_pkt_data: ArrayVec<u8, 64> = ArrayVec::new();
            // let pkt_data: = raw_pkt_data.try_extend_from_slice(&addr.to_be_bytes()).unwrap_or_default(ArrayVec::from([u8 0x10, u8 0x00]));
            let _ = connection.transmit(cec_rs::CecCommand {
                initiator: get_logical_address_or_default(Some(CecLogicalAddress::Playbackdevice1)),
                destination: command.initiator,
                opcode: cec_rs::CecOpcode::GivePhysicalAddress,
                opcode_set: true,
                parameters: cec_rs::CecDatapacket(pkt_data),
                ack: false,
                eom: true,
                transmit_timeout: time::Duration::from_secs(1),
            });
            // physical_address: addr,
            // device_type: CecDeviceType::PlaybackDevice,
            // });
        }
        // Ok(None) => {
        None => {
            error!("No physical address found in connection config");
        } // Err(e) => {
          //     error!("Failed to get physical address: {:?}", e);
          // }
    }
    // } else {
    //     error!("Failed to open CEC connection");
    // }
    // });
}

/// Handle `CecOpcode::VendorCommand`
///
/// This function handles the `cec_rs::CecOpcode::VendorCommand` by responding
/// with a `cec_rs::CecOpcode::FeatureAbort`, and parameters:
///   - Feature Opcode: `cec_rs::CecOpcode::VendorCommand`
///   - Abort Reason: `cec_rs::CecAbortReason::UnrecognizedOpcode`
fn handle_vendor_command(command: &CecCommand, connection: &CecConnection) {
    info!(
        "handle_vendor_command: Received VendorCommand from {:?} with parameters: {:#06x?}",
        command.initiator,
        command.parameters.0.as_slice(),
    );
    let mut pkt_data: ArrayVec<u8, 64> = ArrayVec::new();
    pkt_data.push(cec_rs::CecOpcode::VendorCommand as u8);
    pkt_data.push(cec_rs::CecAbortReason::UnrecognizedOpcode as u8);
    let _ = connection.transmit(cec_rs::CecCommand {
        initiator: get_logical_address_or_default(Some(CecLogicalAddress::Playbackdevice1)),
        destination: command.initiator,
        opcode: cec_rs::CecOpcode::FeatureAbort,
        opcode_set: true,
        parameters: cec_rs::CecDatapacket(pkt_data.clone()),
        ack: false,
        eom: true,
        transmit_timeout: time::Duration::from_secs(1),
    });
    debug!(
        "handle_vendor_command: Sent FeatureAbort with opcode VendorCommand, reason UnrecognizedOpcode to {:?} ({})",
        command.initiator,
        pkt_data
            .iter()
            .map(|b| format!("{:02x?}", b))
            .collect::<Vec<_>>()
            .join(":"),
    )
}

/// Handle `CecOpcode::DeviceVendorId`
///
/// This function handles the `cec_rs::CecOpcode::DeviceVendorId` command by
/// simply logging a friendly vendor ID in both `String` and the resulting `u32`
/// converted from the raw command bytes.
///
/// ## Known Issues
///
/// - The internal `VENDOR_IDS` lookup table is hardcoded with values from
///   lower-level library `libcec_sys::cec_vendor_id_*`.
/// - It would be better if the `cec_rs::CecVendorId` `enum` implemented `From`
///   or `TryFrom` traits instead. Then, we could compare against this type.
/// - The lower-level `libcec` code already logs the vendor id in a different
///   format. For example:
///
///       [DEBUG] (2) cec_dpms: libcec: >> TV (0) -> Broadcast (F): device vendor id (87)
///       [DEBUG] (2) cec_dpms: libcec: << Playback 1 (4) -> Broadcast (F): vendor id LG (e091)
///
fn handle_device_vendor_id(command: &CecCommand) {
    // TODO: Implement external TryFrom trait for cec_rs::CecVendorId
    // For now, this will have to do...
    const VENDOR_IDS: &[(&str, u32)] = &[
        ("LG", libcec_sys::cec_vendor_id_LG),
        ("Samsung", libcec_sys::cec_vendor_id_SAMSUNG),
        ("Sony", libcec_sys::cec_vendor_id_SONY),
        ("Panasonic", libcec_sys::cec_vendor_id_PANASONIC),
        ("Vizio", libcec_sys::cec_vendor_id_VIZIO),
    ];

    let vendor_bytes = command.parameters.0.as_slice();
    // Hack: Assume vendor_bytes[0..3] is enough to identify all possible vendors
    // According to cec-o-matic, the max value for a vendor id is: 16777215
    // This translates to FF:FF:FF in hex
    // For example raw CEC command: 0F:87:ff:ff:ff
    // So, this assumption holds, since we should always have 3 bytes
    if vendor_bytes.len() == 3 {
        let vendor_id = u32::from_be_bytes([0, vendor_bytes[0], vendor_bytes[1], vendor_bytes[2]]);
        let (vendor_name, vendor_id) = (
            VENDOR_IDS
                .iter()
                .find(|(_, id)| *id == vendor_id)
                .map(|(name, _)| name.to_string())
                .unwrap_or_else(|| format!("Unknown (0x{:06x})", vendor_id)),
            vendor_id,
        );
        info!(
            "Device vendor ID from {:?}: {} ({} = 0x{:04x})",
            command.initiator, vendor_name, vendor_id, vendor_id
        );
    } else {
        let vendor_name = "Unknown (insufficient data)";
        let vendor_id = vendor_bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<Vec<_>>()
            .join(":");
        info!(
            "Device vendor ID from {:?}: {} ({})",
            command.initiator, vendor_name, vendor_id
        );
    };
}

fn get_logical_address_or_default(default_addr: Option<CecLogicalAddress>) -> CecLogicalAddress {
    // Fallback default if default_addr was None
    let default = CecLogicalAddress::Playbackdevice1;

    // CONNECTION.with(|conn| {
    // Get mutable access to the thread_local RefCell contents and set it
    // *conn.borrow_mut() = cfg.open().ok();
    // connection = cfg.open().unwrap();
    // if let Some(connection) = conn.borrow().as_ref() {
    if let Some(Some(connection)) = CONNECTION.get() {
        connection.get_logical_addresses()
                .map(|addrs| addrs.primary.into())
                .unwrap_or_else(|e| {
                    warn!(
                        "<b><yellow>Warn:</> could not detect my logical address. Using default: {:#?}\n{:?}",
                        default_addr.unwrap_or(default),
                        e
                    );
                    default_addr.unwrap_or(default)
                })
    } else {
        default_addr.unwrap_or(default)
    }
    // })
}

/// Returns the hostname of the current system, for use with CEC `OSD Name`.
///
/// This function gets the system hostname and returns it, or in the case of a
/// retrieval error, the string "`dummy`".  Although intended for use with CEC
/// `OSD Name`, it does not truncate the returned string. The string will be
/// truncated to 14 bytes by `libcec-sys` (not including C-string trailing null)
/// when setting a CEC `OSD Name` with `device_name()`.  It's not necessary to
/// append a trailing null, as this is done by lower-level `libcec` C bindings.
///
/// ## Example
///
/// ```rust
/// # use std::io;
/// # fn main() -> io::Result<()> {
/// let name = get_osd_hostname();
/// # Ok(())
/// # }
/// ```
///
/// ## Errors
///
/// If the `hostname::get()` function encounters any form of error, the default
/// string, "`dummy`", will be returned; in practice this is rare to happen.
///
/// If the returned hostname contains non-Unicode characters, this is a fatal
/// error, and the program panics.
/// This should **_not_** be possible according to Internet Standards:
/// [RFC 952][1], [RFC 921][2], [RFC 1123][3], and [RFC 3492][4]
///
/// [1]: https://www.rfc-editor.org/rfc/rfc952
/// [2]: https://www.rfc-editor.org/rfc/rfc921
/// [3]: https://www.rfc-editor.org/rfc/rfc1123
/// [4]: https://www.rfc-editor.org/info/rfc3492
fn get_osd_hostname() -> String {
    let hostname_result = hostname::get();
    match hostname_result {
        Err(e) => {
            debug!("get_osd_hostname: Error getting hostname {}", e);
            "dummy".to_string() // Just use a default value
        }
        Ok(v) => {
            debug!("get_osd_hostname: Hostname {:?}", v);
            v.into_string()
                .expect("Hostname should not contain non-Unicode chars")
        }
    }
}

static CONNECTION: std::sync::OnceLock<Option<Arc<CecConnection>>> = std::sync::OnceLock::new();
thread_local! {
    static THREAD_CONNECTION: RefCell<Option<Arc<CecConnection>>> = RefCell::new(None);
}

fn main() -> Result<(), Box<dyn Error>> {
    let old_thread_count = GLOBAL_THREAD_COUNT.fetch_add(1, Ordering::Relaxed);
    debug!("live threads at start of main(): {}", old_thread_count + 1);
    let args = Args::parse();
    logging_init(args.debug);
    // let device_path = args.input.unwrap().into_os_string().into_string().unwrap();
    let device_path = args.input.unwrap().into_os_string().into_encoded_bytes();
    let cstring_device_path = unsafe {
        // SAFETY: We know device_path is a valid path string without interior NUL bytes
        CString::from_vec_unchecked(device_path)
    };

    info!(
        "🔘 <b>cec-dpms</> started, about to open CEC connection to: <u>{:?}</>",
        &cstring_device_path
    );

    let hostname = get_osd_hostname();
    info!("Hostname: <b>{:?}</>", hostname);
    let cfg = CecConnectionCfgBuilder::default()
        .port(cstring_device_path)
        .device_name(hostname.into())
        .activate_source(true)
        // .base_device(CecLogicalAddress::Unknown)
        .base_device(CecLogicalAddress::Tv)
        .physical_address(CEC_INVALID_PHYSICAL_ADDRESS.try_into().unwrap())
        // .physical_address(0x3000)
        .hdmi_port(3)
        .command_received_callback(Box::new(on_command_received))
        .log_message_callback(Box::new(on_log_message))
        .device_types(CecDeviceTypeVec::new(CecDeviceType::PlaybackDevice))
        .adapter_type(cec_rs::CecAdapterType::P8External)
        .build()
        .unwrap();
    // Setup signal handling flags
    let usr1 = Arc::new(AtomicBool::new(false));
    let usr2 = Arc::new(AtomicBool::new(false));
    let terminate = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(SIGUSR1, Arc::clone(&usr1))?;
    signal_hook::flag::register(SIGUSR2, Arc::clone(&usr2))?;
    signal_hook::flag::register(SIGTERM, Arc::clone(&terminate))?;
    signal_hook::flag::register(SIGINT, Arc::clone(&terminate))?;

    // Open CecConnection directly
    let connection = match cfg.open() {
        Ok(conn) => {
            info!("Successfully opened CEC connection");
            Some(Arc::new(conn))
        }
        Err(e) => {
            error!("Failed to open CEC connection: {:?}", e);
            None
        }
    };

    // Store in static for access from callbacks and main thread
    let _ = CONNECTION.set(connection.clone());

    // Also store in thread-local for main thread use
    THREAD_CONNECTION.with(|tconn| {
        *tconn.borrow_mut() = connection.clone();
    });

    // Verify connection is working
    let _res = connection
        .map(|conn| {
            info!(
                "Am I active source? <b>{:?}</>",
                conn.is_active_source(CecLogicalAddress::Playbackdevice1)
            );
            info!("Active source: <b>{:?}</>", conn.get_active_source());
            Ok(()) as Result<(), Box<dyn Error>>
        })
        .unwrap_or_else(|| {
            let err_msg = "Failed to open CEC connection";
            error!("{}", err_msg);
            Err(err_msg.to_string().into())
        });

    let last_thread_count = &GLOBAL_THREAD_COUNT.load(Ordering::Relaxed);
    debug!("live threads at start of main(): {}", last_thread_count);
    info!("Waiting for signals...");
    loop {
        if usr1.load(Ordering::Relaxed) {
            info!("<b><green>USR1</>: powering <b>ON</>");
            let last_thread_count = &GLOBAL_THREAD_COUNT.load(Ordering::Relaxed);
            debug!("live threads at USR1 handler start: {}", last_thread_count);
            usr1.store(false, Ordering::Relaxed);
            // This apparently set active source to Tv??
            // let _ = connection.send_power_on_devices(CecLogicalAddress::Tv);
            let _res = THREAD_CONNECTION.with(|conn| {
                if let Some(connection) = conn.borrow().as_ref() {
                    let power_on_devices_result =
                        connection.send_power_on_devices(CecLogicalAddress::Tv);
                    match power_on_devices_result {
                        Ok(()) => {
                            info!("<b><green>Success!</> Sent power on command to Tv");
                        }
                        Err(e) => {
                            error!(
                                "<b><red>Error:</> Failed to send power on devices command! {:?}",
                                e
                            );
                        }
                    }
                    info!("Active source: <b>{:?}</>", connection.get_active_source());
                    // Do not change active source if Tv reports another source is active
                    // In other words: user viewing preference overrides, and
                    // accidentally bumping the mouse or keyboard won't
                    // deactivate another source
                    //the following call is working the same on my samsung, idk what is more proper:
                    if connection.is_active_source(CecLogicalAddress::Playbackdevice1) {
                        let set_active_source_result: Result<(), cec_rs::CecConnectionResultError> =
                            connection.set_active_source(CecDeviceType::PlaybackDevice);
                        match set_active_source_result {
                            Ok(o) => {
                                info!("<b><green>Success!</> Set active source {:?}", o);
                            }
                            Err(e) => {
                                error!("<b><red>Error:</> Failed to set active source {:?}!", e);
                            }
                        }
                    } else {
                        info!("<b><yellow>Playbackdevice1</> was not active source... skipping");
                    }
                    info!(
                        "<i>connection.get_logical_addresses()</i> = {:?}",
                        connection.get_logical_addresses()
                    );
                    Ok(()) as Result<(), Box<dyn Error>>
                } else {
                    let err_msg = "Failed to open CEC connection";
                    error!("{}", err_msg);
                    Err(err_msg.to_string().into())
                }
            });
        }
        if usr2.load(Ordering::Relaxed) {
            info!("<b><green>USR2</>: powering <b>OFF</>");
            usr2.store(false, Ordering::Relaxed);
            THREAD_CONNECTION.with(|conn| {
                // Get mutable access to the thread_local RefCell contents and set it
                // *conn.borrow_mut() = cfg.open().ok();
                if let Some(connection) = conn.borrow().as_ref() {
                    info!(
                        "<b><green>Active source:</> <b>{:?}</>",
                        connection.get_active_source()
                    );
                    if connection.is_active_source(CecLogicalAddress::Playbackdevice1) {
                        let _ = connection.send_standby_devices(CecLogicalAddress::Tv);
                    } else {
                        info!("<i>reguest ignored</>: we are not an active source");
                    }
                    Ok(()) as Result<(), Box<dyn Error>>
                } else {
                    let err_msg = "Failed to open CEC connection";
                    error!("{}", err_msg);
                    Err(err_msg.to_string().into())
                }
            })?;
        }
        if terminate.load(Ordering::Relaxed) {
            info!("Terminating");
            break;
        }
        thread::sleep(time::Duration::from_secs(1));
    }
    Ok(()) as Result<(), Box<dyn Error>>

    // Ok(())
}
