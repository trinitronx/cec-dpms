# Graceful Shutdown with systemd Integration

## Overview

cec-dpms implements proper systemd shutdown semantics via the `sd-notify` protocol. When the service receives SIGTERM from systemd, it immediately notifies systemd that it's shutting down, allowing clean termination without SIGKILL escalation.

## The Problem (Without Graceful Shutdown)

When systemd issues SIGTERM but doesn't receive a confirmation that the service is responding:

```
[systemd sends SIGTERM]
         ↓
[systemd starts TimeoutStopSec timer]
         ↓
[Service processes signal (internal cleanup)]
[systemd is unaware service is shutting down]
         ↓
[TimeoutStopSec expires, systemd loses patience]
         ↓
systemd: State 'stop-sigterm' timed out. Killing.
[SIGKILL sent - forced termination]
```

Result: Service logs report SIGKILL, no graceful cleanup.

## The Solution (With systemd notify)

With proper systemd integration:

```
[systemd sends SIGTERM]
         ↓
[Service receives signal immediately]
         ↓
[Service sends STOPPING=1 to systemd]
         ↓
systemd: Service is shutting down (got confirmation)
         ↓
[Service performs cleanup]
         ↓
[Service exits with code 0 within TimeoutStopSec]
         ↓
systemd: Service stopped gracefully
```

Result: Clean shutdown, proper resource cleanup, service exit logged as successful.

## Implementation Details

### Multi-Level Signal Handling

The service handles SIGTERM at multiple levels to ensure responsiveness even during long-running operations:

1. **Loop Entry Check** - Checks before entering any signal handler
2. **Handler Entry Check** - Immediate check when USR1/USR2 handler starts
3. **Mid-Operation Checks** - Checks between major CEC operations
4. **Final Exit** - Graceful cleanup before process exit

This ensures systemd receives the STOPPING notification promptly, even if CEC operations are in progress.

### Code Changes

When SIGTERM is received:

```rust
if terminate.load(Ordering::Relaxed) {
    info!("<b><yellow>SIGTERM/SIGINT received, shutting down gracefully...</>");
    notify_systemd_stopping();  // Send STOPPING=1 to systemd
    break;                      // Exit main loop for cleanup
}
```

### Notification Functions

**With systemd feature enabled:**
```rust
fn notify_systemd_stopping() {
    match sd_notify::notify(false, &[NotifyState::Stopping]) {
        Ok(_) => {
            debug!("systemd: Service stopping notification sent");
        }
        Err(e) => {
            debug!("systemd: Failed to send stopping notification: {:?}", e);
        }
    }
}
```

**Without systemd feature:**
```rust
fn notify_systemd_stopping() {
    debug!("systemd feature disabled, skipping stopping notification");
}
```

On non-systemd systems, this is a no-op and causes no issues.

## Service File Configuration

The systemd service unit must be configured to use the notify type:

```ini
[Service]
Type=notify
NotifyAccess=main
TimeoutStopSec=10
```

### Key Parameters

**`Type=notify`**
- Tells systemd the service will send status notifications
- systemd expects notification protocol messages
- Required for graceful shutdown integration

**`NotifyAccess=main`**
- Only the main process can send notifications
- Prevents child processes from confusing systemd
- Standard for most services

**`TimeoutStopSec=10`**
- Maximum time for graceful shutdown
- Gives service 10 seconds to clean up and exit
- Only escalates to SIGKILL if timeout is exceeded
- With proper notify integration, SIGKILL rarely happens

## Shutdown Sequence

### 1. Signal Reception (Immediate)
```
systemd → SIGTERM → cec-dpms process
```

### 2. Notification Dispatch (< 1ms)
```
cec-dpms → STOPPING=1 → systemd
systemd receives confirmation that shutdown is in progress
```

### 3. Graceful Cleanup
```
cec-dpms:
  - Exits main signal loop
  - Closes CEC connection
  - Flushes logs
  - Performs cleanup
  - Exits with code 0
```

### 4. systemd Verification
```
systemd:
  - Monitors process exit
  - Logs successful shutdown
  - Proceeds with next service in sequence
```

Total time: ~100-500ms on healthy system

## Benefits

### For systemd
- Clear communication: service is responding and shutting down
- No timeout confusion or escalation to SIGKILL
- Proper ordering of service dependencies
- Better logging of shutdown sequence

### For Users
- Clean service logs (no SIGKILL)
- Proper resource cleanup (CEC connection closed gracefully)
- Predictable shutdown times
- Better integration with systemd monitoring

### For Developers
- Clear shutdown semantics
- No zombie processes or hanging resources
- Better integration with systemd ecosystem
- Follows systemd best practices

## Troubleshooting

### Still seeing "State 'stop-sigterm' timed out. Killing"

**Cause 1: Service not built with systemd feature**
```bash
# Check if built with systemd support
strings target/release/cec-dpms | grep "systemd"
```

**Solution:**
```bash
cargo build --release --features systemd
```

**Cause 2: Service unit configured as Type=simple**
```bash
systemctl cat cec-dpms | grep "Type="
```

**Solution:** Update service file:
```ini
[Service]
Type=notify
NotifyAccess=main
```

**Cause 3: systemd not running or NOTIFY_SOCKET not set**
```bash
# Check if systemd is running
ps aux | grep systemd
```

**Solution:** Run service under systemd:
```bash
systemctl start cec-dpms
```

### Service exits but takes 10+ seconds

**Cause:** Service taking too long to clean up
```bash
# Check logs for slowness
journalctl -u cec-dpms -n 50 | tail
```

**Solutions:**
1. Increase TimeoutStopSec in service file
2. Profile service to find slow shutdown code
3. Check for blocked I/O operations

### Seeing "systemd feature disabled" messages

**Cause:** Service built without systemd feature
```bash
# Build with systemd support
cargo build --release --features systemd
sudo systemctl restart cec-dpms
```

## Performance Characteristics

### Notification Overhead
- Sending STOPPING notification: < 1ms
- systemd processing notification: < 1ms
- Total: negligible impact on shutdown time

### Memory
- Systemd notify: minimal, compiled-in code only when feature enabled
- No runtime allocation for notifications
- Same binary size as before when compiled

## Systemd Journal Integration

### Successful Graceful Shutdown
```
Dec 29 08:15:30 host cec-dpms[12345]: SIGTERM/SIGINT received, shutting down gracefully...
Dec 29 08:15:30 host cec-dpms[12345]: systemd: Service stopping notification sent
Dec 29 08:15:30 host cec-dpms[12345]: Service shutdown complete
Dec 29 08:15:30 host systemd[1]: cec-dpms.service: Main process exited, code=exited, status=0/SUCCESS
Dec 29 08:15:30 host systemd[1]: cec-dpms.service: Stopped CEC DPMS Power Control
```

### Forced Termination (SIGKILL)
```
Dec 29 08:15:30 host cec-dpms[12345]: SIGTERM/SIGINT received, shutting down gracefully...
Dec 29 08:15:40 host systemd[1]: cec-dpms.service: State 'stop-sigterm' timed out. Killing.
Dec 29 08:15:40 host systemd[1]: cec-dpms.service: Main process exited, code=killed, status=9/KILL
Dec 29 08:15:40 host systemd[1]: cec-dpms.service: Failed with result 'timeout'
```

The first case (successful graceful shutdown) is what you should see with proper configuration.

## Building for Graceful Shutdown

```bash
# For systemd-based systems:
cargo build --release --features systemd
sudo install -D target/release/cec-dpms /usr/bin/cec-dpms

# Install service file:
sudo install -D systemd/cec-dpms.service /etc/systemd/system/

# Enable and start:
sudo systemctl daemon-reload
sudo systemctl enable cec-dpms
sudo systemctl start cec-dpms

# Verify:
sudo systemctl status cec-dpms
```

## Comparison: Before and After

### Before (Without Graceful Shutdown)
```
systemctl stop cec-dpms
State 'stop-sigterm' timed out. Killing.
Failed with result 'timeout'
```

### After (With Graceful Shutdown)
```
systemctl stop cec-dpms
Stopped CEC DPMS Power Control
```

Much cleaner!
