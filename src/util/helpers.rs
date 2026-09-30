use log::debug;

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
pub fn get_osd_hostname() -> String {
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
