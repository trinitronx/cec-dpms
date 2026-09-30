# systemd Integration for cec-dpms

## Overview

The cec-dpms service can optionally integrate with systemd via the `sd-notify` protocol to provide health status notifications. This is controlled by the `systemd` feature flag.

### Building with systemd Support
```bash
cargo build --release --features systemd
```

### Building Without systemd (Default)
```bash
cargo build --release
# or explicitly:
cargo build --release --no-default-features
```

When built without systemd support, notification functions become no-ops that log their intent but do nothing. This enables systemd to:

1. **Know when the service is ready** - systemd waits for `READY=1` before considering the service started
2. **Detect failure conditions** - systemd is notified when the CEC adapter fails and requires intervention

## Service Lifecycle & State Notifications

### Startup → Ready
```
[Service starting]
 ↓
[CEC connection established]
 ↓
READY=1 → systemd
 ↓
[Service running normally]
```

### Normal Operation
The service monitors CEC transmission success/failure ratio and tracks failures cumulatively.

### Graceful Shutdown (SIGTERM)
```
[systemd sends SIGTERM]
 ↓
[Service receives signal]
 ↓
STOPPING=1 → systemd
 ↓
[Clean shutdown in progress]
 ↓
[Service exits with code 0]
```

This notification prevents systemd from timing out waiting for the service to stop, eliminating SIGKILL escalation.

### Failure Detection
```
[Transmission failures accumulating]
 ↓
[5 consecutive failures]
 ↓
[Reconnection attempt fails]
 ↓
STOPPING=1
STATUS=CEC adapter communication failure: {details}
→ systemd
 ↓
[systemd takes configured action: restart, stop, etc.]
```

## systemd Configuration

To enable proper integration, your systemd service unit (`/etc/systemd/system/cec-dpms.service`) should include:

```ini
[Unit]
Description=CEC DPMS Power Control
After=network.target

[Service]
Type=notify
NotifyAccess=main
ExecStart=/usr/bin/cec-dpms --input /dev/cec0 --config /etc/cec-dpms/config.yaml
TimeoutStopSec=10
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=cec-dpms

[Install]
WantedBy=multi-user.target
```

### Key Options

- **`Type=notify`** - Service uses systemd sd\_notify protocol
- **`NotifyAccess=main`** - Only the main process can send notifications
- **`TimeoutStopSec=10`** - Maximum time for graceful shutdown
  - Service sends `STOPPING=1` immediately when receiving SIGTERM
  - systemd waits this duration for clean exit before sending SIGKILL
  - Prevents timeout errors with proper notification
- **`Restart=on-failure`** - Automatically restart if service exits abnormally
- **`RestartSec=5`** - Wait 5 seconds before restarting

### Graceful Shutdown with systemd notify

**Before (without notify):**
```
SIGTERM → Service processes signal
(systemd doesn't know if service is responding)
[Wait TimeoutStopSec]
systemd: State 'stop-sigterm' timed out. Killing.
SIGKILL → Forced termination
```

**After (with notify):**
```
SIGTERM → Service receives signal
Service sends STOPPING=1 immediately
(systemd knows service is shutting down)
Service exits cleanly within TimeoutStopSec
(no SIGKILL needed)
```

The `STOPPING=1` notification ensures systemd knows the service is responding and shutting down gracefully, preventing the timeout escalation to SIGKILL.

## Monitoring Service Health

### Check if Service Reached Ready State
```bash
systemctl status cec-dpms
```

Look for:
- `Active: active (running)` with green dot = Service reached READY=1
- `Active: activating` = Still waiting for READY=1
- `Active: failed` = Service failed after startup

### Check Service Logs
```bash
journalctl -u cec-dpms -f
```

Watch for:
- `systemd: Service ready notification sent` = Connection successful
- `systemd: Service failure notification sent` = CEC adapter failure detected
- `Reconnection failed after X consecutive transmission failures` = Manual intervention needed

### Manual Service Recovery
If the service enters failure state:

1. Check physical connections (TV, CEC adapter, HDMI)
2. Power cycle the TV and/or CEC adapter
3. Restart the service:
   ```bash
   sudo systemctl restart cec-dpms
   ```

## Implementation Details

### Dependencies
- `sd-notify` - Rust bindings for systemd sd\_notify protocol

### Notification Functions
- `notify_systemd_ready()` - Sends READY=1 after successful connection
- `notify_systemd_failure(msg)` - Sends STOPPING + STATUS when recovery fails

### Notification Triggers

**Ready Notification (Sent Once)**
- After initial CEC connection verified and active source status checked

**Failure Notification (Sent When)**
- After 5+ consecutive transmission failures
- Reconnection attempt fails to restore communication

## Graceful Degradation

If the service is not running under systemd (or systemd is not available):
- `sd_notify` calls degrade gracefully and log warnings
- Service continues to function normally
- No systemd-specific errors occur

This allows the service to run in non-systemd environments without modification.
