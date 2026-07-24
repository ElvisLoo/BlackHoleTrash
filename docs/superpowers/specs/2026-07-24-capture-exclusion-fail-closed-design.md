# Capture Exclusion Fail-Closed Design

## Goal

Prevent recursive desktop-capture feedback on Windows by treating capture
exclusion as a required startup capability. A compatible system must behave
exactly as it does today. An incompatible system must keep the overlay hidden,
stop startup, and show one actionable diagnostic dialog.

## Problem

Black Hole Trash captures the desktop and uses that frame as the input to its
fullscreen shader overlay. The overlay therefore has to be excluded from
Windows.Graphics.Capture. If exclusion is ineffective, the renderer consumes
its own previous output and produces repeated holes, nested desktop images, and
rapid color distortion.

The current Windows implementation calls `SetWindowDisplayAffinity` with
`WDA_EXCLUDEFROMCAPTURE`, but ignores the return value and does not read the
affinity back. Capture also starts before that unchecked call. A machine-specific
API or compositor failure can therefore enter the unsafe render loop without a
diagnostic.

## Selected Approach

Use deterministic Win32 validation at two lifecycle points:

1. After the DX12 surface is configured, apply
   `WDA_EXCLUDEFROMCAPTURE` and immediately read it back with
   `GetWindowDisplayAffinity`.
2. Start the monitor capture session only after the first validation succeeds.
3. Immediately after the hidden overlay window is shown and its position,
   size, and topmost level are reasserted, read the affinity back again.

Both validations require the reported affinity to equal exactly
`WDA_EXCLUDEFROMCAPTURE` (`0x11`). Success adds no dialog, notification, or
configuration change.

Runtime image analysis is deliberately excluded from this version. It could
detect a capture implementation that reports affinity correctly but ignores
it, but a reliable image canary needs a separate design to avoid false
positives.

## Components

### Capture exclusion validation

The Windows capture-exclusion wrapper will own the unsafe Win32 calls and
return a structured result instead of discarding their outcomes. It will
distinguish these failures:

- `SetWindowDisplayAffinity` returned failure.
- `GetWindowDisplayAffinity` returned failure.
- The read-back affinity did not equal `0x11`.

The error records the lifecycle stage, failed operation, Win32 error code when
available, requested affinity, and reported affinity when available. Pure
validation and message construction remain separable from the OS calls so they
can be unit tested.

### Startup ordering

Pane initialization keeps the existing order required by DX12, but is split
into two phases:

1. Create every hidden top-level window.
2. Create and configure every DX12 surface.
3. Apply click-through behavior to every window.
4. Apply and verify capture exclusion for every window.
5. Create the per-pane shared frame state.
6. Only after every pane passes, publish the pane set and start each
   Windows.Graphics.Capture session.

Panes are staged locally until all validations succeed. The first failure
aborts state construction, so no overlay window has been shown and no capture
session has been started for any pane.

### Post-show verification

When a pane first becomes visible, the event loop will keep the current
pre-render behavior, show and reposition the window, reassert its topmost
level, and then verify the reported affinity. On failure it will immediately
hide every overlay pane, show one diagnostic dialog, request event-loop exit,
and skip the rest of that event iteration.

Monitor-selection rebuilds use the same checked pane-construction path. A
failure while rebuilding also hides the overlay, reports once, and exits.

### Diagnostic dialog

The failure path uses a native Windows error dialog that remains visible even
for a release GUI-subsystem build. The dialog includes:

- that Black Hole Trash stopped to prevent recursive screen feedback;
- whether failure happened during initial setup, post-show verification, or a
  monitor rebuild;
- whether setting, reading, or comparing the affinity failed;
- the expected value (`0x00000011`);
- the actual value when available;
- the Win32 error code when available;
- a short compatibility hint covering Windows 10 version 2004 or newer,
  Desktop Window Manager, remote/virtual sessions, and graphics drivers.

The same diagnostic is written to stderr for debug builds. Only one dialog is
shown for a failure.

## Error Handling

Capture exclusion is fail-closed. The application must not continue with a
live fullscreen overlay after any deterministic exclusion validation failure.
It must not silently downgrade to `WDA_MONITOR`, unchecked capture, or a
partially initialized pane set.

Errors unrelated to capture exclusion retain their existing behavior and are
outside this change.

## Testing

Unit tests cover the deterministic decision and diagnostic data:

- a failed set operation is rejected;
- a failed read operation is rejected;
- a successful read of a value other than `0x11` is rejected and reports both
  values;
- a successful read of exactly `0x11` is accepted;
- diagnostic text names the stage and available Win32 details.

The implementation follows a red-green cycle: add each behavioral test, run it
to confirm the intended failure, add the minimal implementation, and rerun it.

Repository verification consists of:

- `cargo fmt --all -- --check`
- `cargo test --all-targets`
- `cargo check --all-targets`
- `cargo run --example validate_shader`
- `cargo build --release --bin BlackHoleTrash`

Manual validation on a compatible Windows machine confirms that successful
startup produces no new UI. The injected/pure failure cases validate dialog
content without intentionally starting an unsafe recursive capture loop.

## Non-Goals

- Detecting self-capture by inspecting rendered pixels.
- Automatically changing Windows, DWM, driver, or remote-session settings.
- Continuing with a static wallpaper or synthetic background after failure.
- Changing the existing visual effect, cursor gravity, recycle-bin behavior,
  configuration format, or successful startup UX.
