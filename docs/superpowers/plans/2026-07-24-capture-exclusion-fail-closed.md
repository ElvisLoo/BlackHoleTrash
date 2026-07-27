# Capture Exclusion Fail-Closed Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refuse to start the live desktop overlay when Windows cannot apply and report `WDA_EXCLUDEFROMCAPTURE`, while preserving the current silent success path.

**Architecture:** Add a focused Windows platform module that owns the unsafe display-affinity calls, deterministic validation, diagnostic formatting, and native error dialog. Change pane creation to stage and validate every hidden window before starting any capture thread, then revalidate each window immediately after it becomes visible. Propagate failures to `main` as displayable errors and exit fail-closed.

**Tech Stack:** Rust 2021, winit 0.29, Windows User32 APIs through the existing `windows` and `raw-window-handle` dependencies, Cargo unit tests.

---

## File Structure

- Create `src/platform/windows/capture_exclusion.rs`: Win32 affinity calls, validation model, diagnostic message, popup, and unit tests.
- Modify `src/platform/windows/mod.rs`: export the checked capture-exclusion API.
- Modify `src/main.rs`: reorder capture startup, propagate setup failures, revalidate after show, and exit after one popup.

No dependency, configuration, shader, installer, or release metadata changes are required. All commits remain local; do not run `git push` or create a pull request.

### Task 1: Deterministic Affinity Validation

**Files:**
- Create: `src/platform/windows/capture_exclusion.rs`
- Modify: `src/platform/windows/mod.rs`
- Test: `src/platform/windows/capture_exclusion.rs`

- [ ] **Step 1: Write the failing validation tests and compile-time API skeleton**

Create the module with the required constant, observation model, error shape, a `todo!()` validation body, and these tests:

```rust
pub const REQUIRED_AFFINITY: u32 = 0x11;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AffinityObservation {
    SetFailed(u32),
    ReadFailed(u32),
    Reported(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureExclusionError {
    stage: &'static str,
    observation: AffinityObservation,
}

fn validate_affinity(
    stage: &'static str,
    observation: AffinityObservation,
) -> Result<(), CaptureExclusionError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_exact_exclude_from_capture_value() {
        assert_eq!(
            validate_affinity("initial setup", AffinityObservation::Reported(0x11)),
            Ok(())
        );
    }

    #[test]
    fn rejects_set_failure_with_error_code() {
        let error = validate_affinity(
            "initial setup",
            AffinityObservation::SetFailed(87),
        )
        .unwrap_err();
        assert_eq!(error.stage, "initial setup");
        assert_eq!(error.observation, AffinityObservation::SetFailed(87));
    }

    #[test]
    fn rejects_read_failure_with_error_code() {
        let error = validate_affinity(
            "post-show verification",
            AffinityObservation::ReadFailed(5),
        )
        .unwrap_err();
        assert_eq!(
            error.observation,
            AffinityObservation::ReadFailed(5)
        );
    }

    #[test]
    fn rejects_mismatched_reported_value() {
        let error = validate_affinity(
            "monitor rebuild",
            AffinityObservation::Reported(0x01),
        )
        .unwrap_err();
        assert_eq!(
            error.observation,
            AffinityObservation::Reported(0x01)
        );
    }
}
```

Add `mod capture_exclusion;` to `src/platform/windows/mod.rs`. Do not export or call it yet.

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```powershell
cargo test capture_exclusion::tests -- --nocapture
```

Expected: all four tests fail at `not yet implemented`, proving the new behavior is not implemented.

- [ ] **Step 3: Implement the minimal validation decision**

Replace the `todo!()` body with:

```rust
match observation {
    AffinityObservation::Reported(REQUIRED_AFFINITY) => Ok(()),
    observation => Err(CaptureExclusionError { stage, observation }),
}
```

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run:

```powershell
cargo test capture_exclusion::tests -- --nocapture
```

Expected: 4 passed, 0 failed.

- [ ] **Step 5: Commit the validation model locally**

```powershell
git add -- src/platform/windows/capture_exclusion.rs src/platform/windows/mod.rs
git commit -m "test: define capture exclusion validation"
```

Do not push the commit.

### Task 2: Checked Win32 Calls and Diagnostic Dialog

**Files:**
- Modify: `src/platform/windows/capture_exclusion.rs`
- Modify: `src/platform/windows/mod.rs`
- Test: `src/platform/windows/capture_exclusion.rs`

- [ ] **Step 1: Add failing diagnostic-message tests**

First add this deliberately empty API body so the tests fail through their
assertions rather than through a compiler error:

```rust
impl CaptureExclusionError {
    pub fn user_message(&self) -> String {
        String::new()
    }
}
```

Then add these tests below the Task 1 tests:

```rust
#[test]
fn mismatch_message_contains_stage_and_values() {
    let error = validate_affinity(
        "post-show verification",
        AffinityObservation::Reported(0x01),
    )
    .unwrap_err();
    let message = error.user_message();
    assert!(message.contains("post-show verification"));
    assert!(message.contains("Expected affinity: 0x00000011"));
    assert!(message.contains("Reported affinity: 0x00000001"));
}

#[test]
fn win32_failure_message_contains_operation_and_code() {
    let error = validate_affinity(
        "initial setup",
        AffinityObservation::SetFailed(87),
    )
    .unwrap_err();
    let message = error.user_message();
    assert!(message.contains("SetWindowDisplayAffinity failed"));
    assert!(message.contains("Win32 error: 87"));
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```powershell
cargo test capture_exclusion::tests -- --nocapture
```

Expected: both new tests fail because `user_message()` returns an empty string.

- [ ] **Step 3: Implement diagnostic formatting, window-handle errors, checked calls, and popup**

Complete `capture_exclusion.rs` with these production interfaces and behavior:

```rust
use std::fmt;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::{
    core::{w, HSTRING},
    Win32::{
        Foundation::GetLastError,
        UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK},
    },
};
use winit::window::Window;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureExclusionError {
    stage: &'static str,
    failure: CaptureExclusionFailure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CaptureExclusionFailure {
    WindowHandle(String),
    SetFailed(u32),
    ReadFailed(u32),
    Mismatch(u32),
}

impl CaptureExclusionError {
    fn from_observation(stage: &'static str, observation: AffinityObservation) -> Self {
        let failure = match observation {
            AffinityObservation::SetFailed(code) => CaptureExclusionFailure::SetFailed(code),
            AffinityObservation::ReadFailed(code) => CaptureExclusionFailure::ReadFailed(code),
            AffinityObservation::Reported(actual) => CaptureExclusionFailure::Mismatch(actual),
        };
        Self { stage, failure }
    }

    pub fn user_message(&self) -> String {
        let detail = match &self.failure {
            CaptureExclusionFailure::WindowHandle(detail) => {
                format!("Windows window handle unavailable: {detail}")
            }
            CaptureExclusionFailure::SetFailed(code) => {
                format!("SetWindowDisplayAffinity failed.\nWin32 error: {code}")
            }
            CaptureExclusionFailure::ReadFailed(code) => {
                format!("GetWindowDisplayAffinity failed.\nWin32 error: {code}")
            }
            CaptureExclusionFailure::Mismatch(actual) => format!(
                "Capture exclusion did not match.\nExpected affinity: 0x{REQUIRED_AFFINITY:08X}\nReported affinity: 0x{actual:08X}"
            ),
        };
        format!(
            "Black Hole Trash stopped before recursive screen feedback could start.\n\nStage: {}\n{}\n\nRequired environment: Windows 10 version 2004 or newer with Desktop Window Manager capture exclusion support. Remote sessions, virtual machines, or graphics drivers may not support this mode.",
            self.stage, detail
        )
    }
}

impl fmt::Display for CaptureExclusionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.user_message())
    }
}

fn window_hwnd(window: &Window, stage: &'static str) -> Result<isize, CaptureExclusionError> {
    let handle = window.window_handle().map_err(|error| CaptureExclusionError {
        stage,
        failure: CaptureExclusionFailure::WindowHandle(error.to_string()),
    })?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get()),
        other => Err(CaptureExclusionError {
            stage,
            failure: CaptureExclusionFailure::WindowHandle(format!(
                "expected Win32 handle, got {other:?}"
            )),
        }),
    }
}

fn read_affinity(hwnd: isize, stage: &'static str) -> Result<(), CaptureExclusionError> {
    #[link(name = "user32")]
    extern "system" {
        fn GetWindowDisplayAffinity(hwnd: isize, affinity: *mut u32) -> i32;
    }
    let mut actual = 0;
    if unsafe { GetWindowDisplayAffinity(hwnd, &mut actual) } == 0 {
        return validate_affinity(
            stage,
            AffinityObservation::ReadFailed(unsafe { GetLastError() }.0),
        );
    }
    validate_affinity(stage, AffinityObservation::Reported(actual))
}

pub fn apply_and_verify(
    window: &Window,
    stage: &'static str,
) -> Result<(), CaptureExclusionError> {
    #[link(name = "user32")]
    extern "system" {
        fn SetWindowDisplayAffinity(hwnd: isize, affinity: u32) -> i32;
    }
    let hwnd = window_hwnd(window, stage)?;
    if unsafe { SetWindowDisplayAffinity(hwnd, REQUIRED_AFFINITY) } == 0 {
        return validate_affinity(
            stage,
            AffinityObservation::SetFailed(unsafe { GetLastError() }.0),
        );
    }
    read_affinity(hwnd, stage)
}

pub fn verify(window: &Window, stage: &'static str) -> Result<(), CaptureExclusionError> {
    read_affinity(window_hwnd(window, stage)?, stage)
}

pub fn show_failure_message(message: &str) {
    eprintln!("capture exclusion: {message}");
    let message = HSTRING::from(message);
    unsafe {
        MessageBoxW(
            None,
            &message,
            w!("Black Hole Trash - incompatible capture environment"),
            MB_OK | MB_ICONERROR,
        );
    }
}

pub fn show_failure(error: &CaptureExclusionError) {
    show_failure_message(&error.user_message());
}
```

Replace `validate_affinity` with:

```rust
fn validate_affinity(
    stage: &'static str,
    observation: AffinityObservation,
) -> Result<(), CaptureExclusionError> {
    match observation {
        AffinityObservation::Reported(REQUIRED_AFFINITY) => Ok(()),
        observation => Err(CaptureExclusionError::from_observation(stage, observation)),
    }
}
```

In the three rejection tests from Task 1, replace their final observation
assertions with these failure assertions, respectively:

```rust
assert_eq!(error.failure, CaptureExclusionFailure::SetFailed(87));
assert_eq!(error.failure, CaptureExclusionFailure::ReadFailed(5));
assert_eq!(error.failure, CaptureExclusionFailure::Mismatch(0x01));
```

Export the public API from `src/platform/windows/mod.rs`:

```rust
pub use capture_exclusion::{
    apply_and_verify as apply_capture_exclusion,
    show_failure as show_capture_exclusion_failure,
    show_failure_message as show_capture_exclusion_failure_message,
    verify as verify_capture_exclusion,
};
```

- [ ] **Step 4: Run tests and verify GREEN**

Run:

```powershell
cargo test capture_exclusion::tests -- --nocapture
```

Expected: 6 passed, 0 failed.

- [ ] **Step 5: Check the Windows module**

Run:

```powershell
cargo check --bin BlackHoleTrash
```

Expected: exit code 0 with no warnings from `capture_exclusion.rs`.

- [ ] **Step 6: Commit the checked API locally**

```powershell
git add -- src/platform/windows/capture_exclusion.rs src/platform/windows/mod.rs
git commit -m "feat: validate Windows capture exclusion"
```

Do not push the commit.

### Task 3: Fail-Closed Overlay Lifecycle

**Files:**
- Modify: `src/main.rs:341`
- Modify: `src/main.rs:541`
- Modify: `src/main.rs:639`
- Modify: `src/main.rs:1149`
- Modify: `src/main.rs:1530`
- Modify: `src/main.rs:1831`
- Modify: `src/main.rs:2024`
- Modify: `src/main.rs:2105`

- [ ] **Step 1: Make pane creation return errors and stage all panes before capture**

Replace the current return type in the `State::new` signature:

```rust
async fn new(
    target: &EventLoopWindowTarget<()>,
    monitors: &[winit::monitor::MonitorHandle],
    selection: Option<usize>,
) -> Result<State, String> {
```

Replace the current final block:

```rust
state.finish_panes(shells);
state.update_roam_box();
state.center_px = [
    state.roam_pos[0] + state.roam_size[0] * 0.5,
    state.roam_pos[1] + state.roam_size[1] * 0.5,
];
state
```

with:

```rust
state.finish_panes(shells, "initial setup")?;
state.update_roam_box();
state.center_px = [
    state.roam_pos[0] + state.roam_size[0] * 0.5,
    state.roam_pos[1] + state.roam_size[1] * 0.5,
];
Ok(state)
```

Change the `finish_panes` opening from:

```rust
fn finish_panes(&mut self, shells: Vec<Shell>) {
```

to:

```rust
#[cfg_attr(not(windows), allow(unused_variables))]
fn finish_panes(
    &mut self,
    shells: Vec<Shell>,
    exclusion_stage: &'static str,
) -> Result<(), String> {
    let mut new_panes = Vec::with_capacity(shells.len());
```

Delete the existing early capture start:

```rust
#[cfg(any(windows, target_os = "macos"))]
capture::start(shared.clone(), mon_index);
```

Replace the unchecked exclusion block:

```rust
let _ = window.set_cursor_hittest(false);
#[cfg(any(windows, target_os = "macos"))]
set_capture_exclusion(&window, true);
```

with:

```rust
let _ = window.set_cursor_hittest(false);
#[cfg(windows)]
platform::windows::apply_capture_exclusion(&window, exclusion_stage)
    .map_err(|error| error.to_string())?;
#[cfg(target_os = "macos")]
set_capture_exclusion(&window, true);
```

Change the destination of the existing complete pane initializer:

```rust
self.panes.push(Pane {
```

to:

```rust
new_panes.push(Pane {
```

Finally, insert this after the pane-construction loop and before the function's
closing brace:

```rust
#[cfg(any(windows, target_os = "macos"))]
for pane in &new_panes {
    capture::start(pane.shared.clone(), pane.mon_index);
}
self.panes.extend(new_panes);
Ok(())
```

Every field in the existing `Pane` initializer remains unchanged.

- [ ] **Step 2: Propagate monitor-rebuild failures**

Change `set_selection` to `Result<(), String>`, pass `"monitor rebuild"`, and only update placement after success:

```rust
fn set_selection(
    &mut self,
    target: &EventLoopWindowTarget<()>,
    monitors: &[winit::monitor::MonitorHandle],
    selection: Option<usize>,
) -> Result<(), String> {
    for pane in &self.panes {
        pane.shared.lock().unwrap().monitor_index = usize::MAX;
    }
    self.panes.clear();
    let shells = make_shells(&self.instance, target, monitors, selection);
    self.finish_panes(shells, "monitor rebuild")?;
    self.update_roam_box();
    self.center_px = [
        self.roam_pos[0] + self.roam_size[0] * 0.5,
        self.roam_pos[1] + self.roam_size[1] * 0.5,
    ];
    Ok(())
}
```

- [ ] **Step 3: Handle initial setup failure before the event loop starts**

Replace direct `State::new` assignment with:

```rust
let mut state = match pollster::block_on(State::new(&event_loop, &monitors, current_sel)) {
    Ok(state) => state,
    Err(message) => {
        #[cfg(windows)]
        platform::windows::show_capture_exclusion_failure_message(&message);
        #[cfg(not(windows))]
        eprintln!("overlay setup failed: {message}");
        return;
    }
};
```

- [ ] **Step 4: Revalidate immediately after each window is shown**

In the visibility state machine, show and restyle inside a short mutable-borrow scope, then verify before setting the pane's logical `visible` flag:

```rust
{
    let pane = &mut state.panes[i];
    pane.window.set_visible(true);
    pane.window.set_outer_position(pane.mon_pos);
    let _ = pane.window.request_inner_size(pane.mon_size);
    pane.window.set_window_level(WindowLevel::AlwaysOnTop);
}

#[cfg(windows)]
if let Err(error) = platform::windows::verify_capture_exclusion(
    &state.panes[i].window,
    "post-show verification",
) {
    for pane in &mut state.panes {
        pane.window.set_visible(false);
        pane.visible = false;
    }
    platform::windows::show_capture_exclusion_failure(&error);
    elwt.exit();
    return;
}

state.panes[i].visible = true;
```

- [ ] **Step 5: Exit on tray/config monitor-rebuild failure**

At both `set_selection` call sites, use this pattern before updating `current_sel` or menu checks:

```rust
if let Err(message) = state.set_selection(elwt, &monitors, sel) {
    #[cfg(windows)]
    platform::windows::show_capture_exclusion_failure_message(&message);
    #[cfg(not(windows))]
    eprintln!("overlay setup failed: {message}");
    elwt.exit();
    return;
}
current_sel = sel;
```

- [ ] **Step 6: Remove the unchecked Windows implementation**

Delete the old Windows-only `set_capture_exclusion` function from `main.rs`. Keep the macOS implementation unchanged because this change targets the observed Windows failure and macOS does not expose the same Win32 read-back API.

- [ ] **Step 7: Format and compile the lifecycle changes**

Run:

```powershell
cargo fmt --all
cargo check --all-targets
```

Expected: both commands exit 0. Resolve borrow or cfg errors without changing the fail-closed ordering.

- [ ] **Step 8: Run the full tests**

Run:

```powershell
cargo test --all-targets
```

Expected: all tests pass, including the six capture-exclusion tests.

- [ ] **Step 9: Commit the lifecycle integration locally**

```powershell
git add -- src/main.rs src/platform/windows/capture_exclusion.rs src/platform/windows/mod.rs
git commit -m "fix: stop when capture exclusion is unavailable"
```

Do not push the commit.

### Task 4: Release Verification

**Files:**
- Verify: `src/main.rs`
- Verify: `src/platform/windows/capture_exclusion.rs`
- Verify: `src/platform/windows/mod.rs`
- Verify: `src/black_hole_trash.wgsl`

- [ ] **Step 1: Verify formatting without modifying files**

```powershell
cargo fmt --all -- --check
```

Expected: exit code 0.

- [ ] **Step 2: Run every test target freshly**

```powershell
cargo test --all-targets
```

Expected: exit code 0 and zero failed tests.

- [ ] **Step 3: Check every target freshly**

```powershell
cargo check --all-targets
```

Expected: exit code 0.

- [ ] **Step 4: Validate the shader independently**

```powershell
cargo run --example validate_shader
```

Expected: output contains `shader OK` and exit code 0.

- [ ] **Step 5: Build the release executable**

```powershell
cargo build --release --bin BlackHoleTrash
```

Expected: exit code 0 and a refreshed `target/release/BlackHoleTrash.exe`.

- [ ] **Step 6: Inspect final local state**

```powershell
git status --short
git log -5 --oneline
```

Expected: no uncommitted source changes; local implementation commits are visible. Do not run `git push`.
