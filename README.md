# 🔘 cec-dpms

[![Crates.io](https://img.shields.io/crates/v/cec-dpms.svg)](https://crates.io/crates/cec-dpms)
[![Crates.io](https://img.shields.io/crates/l/cec-dpms.svg)](https://crates.io/crates/cec-dpms)

## Description
This small linux tool is intended to emulate DPMS using [HDMI-CEC](https://en.wikipedia.org/wiki/Consumer_Electronics_Control) interface.
It is using the [libcec](https://github.com/Pulse-Eight/libcec) library via [cec-rs](https://crates.io/crates/cec-rs).
This way it allows to emulate DPMS but for the TV connected to the CEC bus, as the result the TV behaves similar to a regular monitor.
The user however has to run own scripts for controlling it.

## Usage
```
cec-dpms 0.1.0
Simple program to power on/off TV by simulating DPMS feature using HDMI CEC

USAGE:
    cec-dpms [OPTIONS]

OPTIONS:
    -d, --debug            Enable debug info
    -h, --help             Print help information
    -i, --input <INPUT>    input device path/name of CEC device
    -V, --version          Print version information
```

The program is designed to be continuously running in background (Eg. started from systemd service).<br>
It is listening to `USR1` and `USR2` signals:
- `USR1` is powering ON the TV,
- `USR2` is powering OFF the TV

## Example
An example of using this tool along with [Sway](https://swaywm.org/):<br>
In the sway config file configure the `swayidle` like this:
```
exec swayidle \
    timeout 600 'swaymsg "output * dpms off"' \
       resume 'swaymsg "output * dpms on"' \
    timeout 600 'sudo pkill -USR2 cec-dpms' \
       resume 'sudo pkill -USR1 cec-dpms'
```

## Building

### Quick Start
```bash
# Default build (no systemd integration)
cargo build --release

# With systemd support (optional)
cargo build --release --features systemd
```

See [BUILDING.md](BUILDING.md) for detailed build instructions and feature information.

## systemd Integration

The service can optionally integrate with systemd via the `sd-notify` protocol. This feature is:
- **Optional** - build with `--features systemd` to enable
- **Graceful** - works on both systemd and non-systemd systems
- **Status-aware** - reports ready state and failure conditions

A sample service file for systemd is here:<br>
[systemd/cec-dpms.service](https://github.com/manio/cec-dpms/blob/master/systemd/cec-dpms.service)<br>
You need to adjust it for your needs (eg. check the binary path).<br>
After placing the unit file in correct location and reloading systemd, the unit can be started as usual:<br>
`systemctl start cec-dpms.service`<br>

For detailed systemd integration documentation, see:
- [SystemD_Integration.md](SystemD_Integration.md) - SystemD configuration and monitoring
- [Graceful_Shutdown.md](Graceful_Shutdown.md) - Graceful shutdown with `systemd-notify`
- [Signal_Responsiveness.md](Signal_Responsiveness.md) - Signal handling during long operations
