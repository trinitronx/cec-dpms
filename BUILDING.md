# Building cec-dpms

## Overview

cec-dpms can be built with or without systemd support. The systemd feature is optional to support systems that do not use systemd as their init system.

## Feature Flags

### Default Build (No systemd)
```bash
cargo build --release
```

This produces a binary with:
- No systemd integration
- No external systemd dependencies
- Works on any Linux system (OpenRC, runit, etc.)
- Failure notifications are logged but not sent to init system

### With systemd Support
```bash
cargo build --release --features systemd
```

This produces a binary with:
- Full systemd sd_notify integration
- Ready and failure state notifications to systemd
- Requires libsystemd development libraries at runtime
- Ideal for systemd-based distributions

## Build Requirements

### Common Requirements
- Rust 1.56+ (2021 edition)
- libcec development libraries (`libcec-dev` on Debian/Ubuntu)
- Standard build tools (gcc, make)

### For systemd Feature
- libsystemd development libraries (`libsystemd-dev` on Debian/Ubuntu)
- systemd headers

## Installation

### Without systemd (Generic Linux)
```bash
cargo build --release --no-default-features
sudo install -D target/release/cec-dpms /usr/local/bin/cec-dpms
```

### With systemd Support
```bash
cargo build --release --features systemd
sudo install -D target/release/cec-dpms /usr/local/bin/cec-dpms
```

## Runtime Behavior by Feature

### Without systemd Feature
When systemd notification attempts are made:
```
[DEBUG] systemd feature disabled, skipping ready notification
[WARN]  systemd feature disabled, would have notified failure: ...
```

The service:
- Operates normally
- Logs status messages to console/syslog
- Does not communicate with systemd
- Does not require systemd to be running

### With systemd Feature
When systemd is available:
```
[DEBUG] systemd: Service ready notification sent
[WARN]  systemd: Service failure notification sent: ...
```

When systemd is not available (e.g., running in non-systemd environment):
```
[DEBUG] systemd: Failed to send ready notification: ...
[WARN]  systemd: Failed to send failure notification: ...
```

The service still operates normally; notifications simply fail gracefully.

## Dependency Comparison

### Without systemd Feature
Dependencies:
- cec-rs
- simplelog
- signal-hook
- clap
- hostname
- arrayvec
- serde
- serde-saphyr
- directories
- lazy_static

Binary size: ~1.9 MB (stripped)

### With systemd Feature
Additional dependencies:
- sd-notify
- libsystemd-sys

Binary size: ~1.9 MB (stripped) - minimal difference due to feature gating

## Choosing Your Build

| Requirement | Recommendation |
|---|---|
| Running on systemd-based distro (Fedora, Debian 10+, Ubuntu 18.04+, Arch) | `--features systemd` |
| Running on non-systemd system (OpenRC, runit, init) | Default or `--no-default-features` |
| Container/embedded environment | Default or `--no-default-features` |
| Minimal binary size | Default or `--no-default-features` |
| Want systemd integration | `--features systemd` |

## Environment-Specific Build Recommendations

### Arch Linux
```bash
cargo build --release --features systemd
```
Arch uses systemd exclusively.

### Alpine Linux
```bash
cargo build --release --no-default-features
```
Alpine uses OpenRC by default.

### CentOS/RHEL
```bash
cargo build --release --features systemd
```
Uses systemd since version 7+.

### Ubuntu/Debian
```bash
cargo build --release --features systemd
```
Uses systemd since Debian 9+ and Ubuntu 16.04+.

### Non-Linux Systems
cec-dpms currently only supports Linux systems with libcec support.

## Troubleshooting Builds

### Build fails with "error: could not compile `sd-notify`"
**Solution:** You're missing libsystemd development libraries.
```bash
# Debian/Ubuntu
sudo apt install libsystemd-dev

# Fedora/RHEL
sudo dnf install systemd-devel

# Arch
sudo pacman -S systemd
```

### Build fails with "error: could not compile `libcec-sys`"
**Solution:** You're missing libcec development libraries.
```bash
# Debian/Ubuntu
sudo apt install libcec-dev

# Fedora/RHEL
sudo dnf install libcec-devel

# Arch
sudo pacman -S libcec
```

### Runtime error: "libcec.so.6: cannot open shared object file"
**Solution:** libcec runtime library is not installed.
```bash
# Debian/Ubuntu
sudo apt install libcec6

# Fedora/RHEL
sudo dnf install libcec

# Arch
sudo pacman -S libcec
```

## Cargo.toml Configuration

Features are defined as:
```toml
[features]
default = []
systemd = ["sd-notify"]
```

This means:
- By default, no features are enabled
- When `--features systemd` is specified, sd-notify is included
- sd-notify is an optional dependency (only included when feature is enabled)
