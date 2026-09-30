# Signal Responsiveness: The 1-Second Sleep Issue

## Problem

When systemd sends SIGTERM to cec-dpms, the service doesn't respond promptly, resulting in:

```
cec-dpms@dev-ttyACM0.service: State 'stop-sigterm' timed out. Killing.
cec-dpms@dev-ttyACM0.service: Killing process XXXX (cec-dpms) with signal SIGKILL.
```

## Root Cause (SOLVED)

The main loop was sleeping for **1 full second** at the end of each iteration:

```rust
loop {
    // ... process signals ...
    if terminate.load(Ordering::Relaxed) {
        break;
    }
    thread::sleep(time::Duration::from_secs(1));  // ← PROBLEM: 1 second sleep!
}
```

When SIGTERM arrives:

1. Signal handler sets the `terminate` flag (takes microseconds)
2. **But main thread is in `thread::sleep()` - blocked in the kernel**
3. Signal handlers don't interrupt system calls in Rust/Unix
4. Main thread sleeps for up to 1 second before checking the flag
5. systemd's 20-second timeout expires before graceful shutdown completes
6. systemd sends SIGKILL

This is a fundamental Unix design: signal handlers are async-safe but don't interrupt blocking system calls.

## Solution

**Reduce the sleep duration from 1 second to 100 milliseconds.**

```rust
// Before:
thread::sleep(time::Duration::from_secs(1));

// After:
thread::sleep(time::Duration::from_millis(100));
```

### Why This Works

1. **Main loop checks terminate flag at least every 100ms**
   - When SIGTERM arrives, main thread wakes within ~100ms
   - Graceful shutdown begins immediately

2. **Fast response to signals**
   - SIGTERM to shutdown: ~100ms
   - Well within systemd's 20-second timeout: ✓

3. **No impact on system load**
   - Loop runs 10x more frequently but is idle (just checking flags)
   - `signal_hook` handles signals asynchronously (not busy-looping)
   - Minimal CPU impact

### Complete Fix

```rust
loop {
    // Check for termination signal before processing any handlers
    if terminate.load(Ordering::Relaxed) {
        debug!("Terminate signal detected at loop start, breaking early");
        break;
    }

    // ... process USR1/USR2 handlers ...

    if terminate.load(Ordering::Relaxed) {
        info!("<b><yellow>SIGTERM/SIGINT received, shutting down gracefully...</>");
        notify_systemd_stopping();
        break;
    }
    
    // Reduced from 1 second to 100ms for signal responsiveness
    thread::sleep(time::Duration::from_millis(100));
}

// Wait for worker thread to finish
let _ = reconnect_thread.join();

info!("<b><yellow>Service shutdown complete</>");
Ok(())
```

### Process Timeline (Fixed)

```
T0: Main loop runs, processes signals, sleeps for 100ms
T0+50ms: SIGTERM arrives
    Signal handler sets terminate flag
T0+100ms: Main thread wakes from sleep
    Main thread checks terminate flag - sees it's true!
    Main thread calls notify_systemd_stopping()
T0+150ms: Main thread calls join() on worker thread
    Worker thread finishes
T0+150ms: Process exits
    Well within systemd's 20-second timeout
    No SIGKILL
```

## Code Location

File: `src/main.rs` line ~719
- Changed sleep from 1000ms to 100ms
- Added comment explaining the change
- No other code modifications needed

## Testing

```bash
# Rebuild with the fix
cargo build --release --features systemd
sudo cp target/release/cec-dpms /usr/bin/
sudo systemctl daemon-reload

# Start the service
systemctl start cec-dpms@dev-ttyACM0
sleep 2

# Stop it - should exit cleanly within ~100ms
systemctl stop cec-dpms@dev-ttyACM0

# Check logs - should show clean exit
journalctl -u cec-dpms@dev-ttyACM0 -n 15
```

Expected (correct):
```
[INFO] SIGTERM/SIGINT received, shutting down gracefully...
[INFO] Service shutdown complete
cec-dpms@dev-ttyACM0.service: Stopped...
code=exited, status=0/SUCCESS
```

NOT (what we were getting):
```
cec-dpms@dev-ttyACM0.service: State 'stop-sigterm' timed out. Killing.
code=killed, status=9/KILL
```

## Key Learning

**In Unix/Linux, signal handlers don't interrupt blocking system calls.** This is by design for safety and consistency. If you need responsive signal handling:

1. **Keep sleep durations short** - Use milliseconds instead of seconds
2. **Or use signal-safe multiplexing** - Like `pselect()` or `epoll_wait()` with signal masking
3. **Or use async mechanisms** - Like threads with short polling intervals

For simple applications like cec-dpms, the short-sleep approach (100ms) is ideal.
