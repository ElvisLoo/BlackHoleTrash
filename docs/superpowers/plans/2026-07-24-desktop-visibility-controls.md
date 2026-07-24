# Desktop Visibility Controls Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a default desktop-aware visibility mode, an opt-in always-on-top mode, permanent taskbar exclusion, double-Ctrl pointer placement, and a 30 FPS default.

**Architecture:** Keep `main.rs` as the runtime coordinator and isolate Windows API work in three focused modules: `window_occlusion.rs`, `ctrl_double_tap.rs`, and `taskbar.rs`. Pure geometry, filtering, key-sequence, configuration, and FPS policies receive unit tests; runtime integration combines screen-saver readiness with occlusion state and updates rendering, drag/drop, cursor gravity, tray state, and config hot reload atomically.

**Tech Stack:** Rust 2021, winit 0.29, wgpu 23, windows 0.62 Win32 APIs, tray-icon 0.19, inline Rust unit tests.

---

### Task 1: Make 30 FPS the universal default and fallback

**Files:**
- Modify: `src/main.rs`
- Modify: `README.md`
- Modify: `README.en.md`

- [ ] **Step 1: Change the FPS policy tests first**

Update `fps_policy_only_allows_thirty_or_sixty` so `None`, `0`, and unsupported values expect `30`, while explicit `60` remains `60`. Add an assertion that `DEFAULT_FPS == 30`.

```rust
assert_eq!(DEFAULT_FPS, 30);
assert_eq!(normalize_fps(None), 30);
assert_eq!(normalize_fps(Some(0)), 30);
assert_eq!(normalize_fps(Some(30)), 30);
assert_eq!(normalize_fps(Some(60)), 60);
assert_eq!(normalize_fps(Some(120)), 30);
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run: `cargo test fps_policy_only_allows_thirty_or_sixty -- --nocapture`

Expected: FAIL because `DEFAULT_FPS` and current fallback are still `60`.

- [ ] **Step 3: Implement the 30/60 normalization**

```rust
const DEFAULT_FPS: u32 = 30;

fn normalize_fps(value: Option<u32>) -> u32 {
    match value {
        Some(60) => 60,
        _ => DEFAULT_FPS,
    }
}
```

Update the generated config comments and README configuration text so absent and invalid values are documented as falling back to `30`.

- [ ] **Step 4: Run the focused tests**

Run: `cargo test fps_ -- --nocapture`

Expected: all FPS tests PASS.

- [ ] **Step 5: Commit the FPS change**

```powershell
git add src/main.rs README.md README.en.md
git commit -m "feat: default rendering to 30 fps"
```

### Task 2: Add a testable double-Ctrl detector and Windows hook

**Files:**
- Create: `src/platform/windows/ctrl_double_tap.rs`
- Modify: `src/platform/windows/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add failing state-machine tests**

Define tests around a pure `DoubleCtrlState::on_key(vk, pressed, at)` API. Cover two released taps within 350 ms, timeout, repeat keydown, an intervening non-Ctrl key, and Shift/Alt/Win cancellation.

```rust
#[test]
fn two_clean_ctrl_taps_trigger_once() {
    let mut state = DoubleCtrlState::default();
    assert!(!state.on_key(VK_CONTROL.0 as u32, true, ms(0)));
    assert!(!state.on_key(VK_CONTROL.0 as u32, false, ms(40)));
    assert!(state.on_key(VK_CONTROL.0 as u32, true, ms(250)));
    assert!(!state.on_key(VK_CONTROL.0 as u32, true, ms(260)));
}
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run: `cargo test ctrl_double_tap -- --nocapture`

Expected: compilation FAIL because the module and detector do not exist.

- [ ] **Step 3: Implement the pure state machine**

Use `DOUBLE_TAP_WINDOW = 350 ms`, track physical key-down state to reject repeats, record the first Ctrl release, and clear the candidate whenever a non-Ctrl key is pressed. Treat `VK_CONTROL`, `VK_LCONTROL`, and `VK_RCONTROL` as Ctrl.

```rust
#[derive(Default)]
struct DoubleCtrlState {
    keys_down: [bool; 256],
    first_release: Option<Duration>,
    first_press_valid: bool,
}

impl DoubleCtrlState {
    fn on_key(&mut self, vk: u32, pressed: bool, at: Duration) -> bool {
        // Ignore repeats, cancel on any non-Ctrl press, and trigger only on
        // the second clean Ctrl down edge within DOUBLE_TAP_WINDOW.
    }
}
```

- [ ] **Step 4: Wrap the state machine in a low-level keyboard hook**

Implement `CtrlDoubleTapController::new()` with `SetWindowsHookExW(WH_KEYBOARD_LL, ...)`, store hook state in `OnceLock<Arc<Mutex<_>>>`, set an `AtomicBool` when the detector triggers, expose `take_triggered()`, and unhook in `Drop`. Always call `CallNextHookEx`; never suppress input.

```rust
pub struct CtrlDoubleTapController {
    hook: HHOOK,
}

impl CtrlDoubleTapController {
    pub fn take_triggered(&self) -> bool {
        TRIGGERED.swap(false, Ordering::AcqRel)
    }
}
```

Export the controller from `platform::windows`. Remove `place_hotkey_held()` and its Ctrl+Shift polling from `main.rs`; runtime hookup occurs in Task 5.

- [ ] **Step 5: Run tests and commit**

Run: `cargo test ctrl_double_tap -- --nocapture`

Expected: all detector tests PASS.

```powershell
git add src/platform/windows/ctrl_double_tap.rs src/platform/windows/mod.rs src/main.rs
git commit -m "feat: detect double Ctrl placement gesture"
```

### Task 3: Detect ordinary windows covering the black hole

**Files:**
- Create: `src/platform/windows/window_occlusion.rs`
- Modify: `src/platform/windows/mod.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Add failing geometry and filter tests**

Define a small internal `Rect` and a pure `circle_intersects_rect(center, radius, rect)` helper. Test overlap, separation, tangency, containment, and a rectangle occupying only the circle's bounding-box corner. Define `WindowFacts` and test that same-process, Shell, invisible, minimized, cloaked, menu, and tooltip windows are rejected while a normal application window is accepted.

```rust
#[test]
fn bounding_box_corner_does_not_cover_circle() {
    let rect = Rect::new(9.0, 9.0, 12.0, 12.0);
    assert!(!circle_intersects_rect([0.0, 0.0], 10.0, rect));
}
```

- [ ] **Step 2: Run the focused tests and verify they fail**

Run: `cargo test window_occlusion -- --nocapture`

Expected: compilation FAIL because the module does not exist.

- [ ] **Step 3: Implement geometry and candidate filtering**

Clamp the circle center to the rectangle and compare squared distance with squared radius. Reject invalid or empty rectangles. Keep Shell and transient class-name checks in one `is_ignored_class` helper.

```rust
fn circle_intersects_rect(center: [f64; 2], radius: f64, rect: Rect) -> bool {
    let x = center[0].clamp(rect.left, rect.right);
    let y = center[1].clamp(rect.top, rect.bottom);
    let dx = center[0] - x;
    let dy = center[1] - y;
    dx * dx + dy * dy <= radius * radius
}
```

- [ ] **Step 4: Implement Win32 enumeration**

Add the `Win32_Graphics_Dwm` feature. Implement `is_black_hole_occluded(center, radius) -> windows::core::Result<bool>` using `EnumWindows`, `GetWindowThreadProcessId`, `IsWindowVisible`, `IsIconic`, `GetClassNameW`, `DwmGetWindowAttribute(DWMWA_CLOAKED)`, and `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)` with `GetWindowRect` fallback. Stop enumeration early after an intersection and distinguish that deliberate stop from API failure in the callback context.

Export `is_black_hole_occluded` from `platform::windows`.

- [ ] **Step 5: Run tests and commit**

Run: `cargo test window_occlusion -- --nocapture`

Expected: all geometry and filtering tests PASS.

```powershell
git add Cargo.toml src/platform/windows/window_occlusion.rs src/platform/windows/mod.rs
git commit -m "feat: detect windows covering the black hole"
```

### Task 4: Enforce taskbar exclusion for every render Pane

**Files:**
- Create: `src/platform/windows/taskbar.rs`
- Modify: `src/platform/windows/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add style-policy tests**

Extract a pure `excluded_taskbar_style(ex_style)` helper and test that it adds `WS_EX_TOOLWINDOW`, removes `WS_EX_APPWINDOW`, and preserves unrelated bits.

```rust
#[test]
fn taskbar_policy_adds_toolwindow_and_removes_appwindow() {
    let style = excluded_taskbar_style(WS_EX_APPWINDOW.0 as u32 | WS_EX_NOACTIVATE.0 as u32);
    assert_ne!(style & WS_EX_TOOLWINDOW.0 as u32, 0);
    assert_eq!(style & WS_EX_APPWINDOW.0 as u32, 0);
    assert_ne!(style & WS_EX_NOACTIVATE.0 as u32, 0);
}
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run: `cargo test taskbar -- --nocapture`

Expected: compilation FAIL because the module does not exist.

- [ ] **Step 3: Implement application and verification**

Resolve the winit HWND through `raw-window-handle`, read and write `GWL_EXSTYLE` with `GetWindowLongPtrW`/`SetWindowLongPtrW`, call `SetWindowPos(... SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)`, then read back and return an error unless `WS_EX_TOOLWINDOW` is set and `WS_EX_APPWINDOW` is clear.

```rust
pub fn apply_and_verify(window: &Window) -> Result<(), TaskbarExclusionError> {
    let hwnd = hwnd(window)?;
    let desired = excluded_taskbar_style(current_ex_style(hwnd)?);
    set_ex_style(hwnd, desired)?;
    verify_ex_style(hwnd)
}
```

- [ ] **Step 4: Apply exclusion during creation and every show transition**

On Windows, import `WindowBuilderExtWindows` and add `.with_skip_taskbar(true)` before building each Pane. After the wgpu surface exists, call `platform::windows::apply_taskbar_exclusion`; call its verifier again before showing a previously hidden Pane. Treat failure like capture-exclusion failure: hide all Pane windows, show one error dialog, and exit.

- [ ] **Step 5: Run tests and commit**

Run: `cargo test taskbar -- --nocapture`

Expected: style-policy tests PASS.

```powershell
git add src/platform/windows/taskbar.rs src/platform/windows/mod.rs src/main.rs
git commit -m "feat: keep render windows out of the taskbar"
```

### Task 5: Integrate tray, config, visibility, and placement state

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add failing configuration tests**

Extend `FileConfig` with `always_on_top: Option<u32>` and test missing, `0`, `1`, and invalid values. Normalize through one helper so only `1` enables the mode.

```rust
#[test]
fn always_on_top_defaults_off_and_only_one_enables_it() {
    assert!(!normalize_always_on_top(None));
    assert!(!normalize_always_on_top(Some(0)));
    assert!(normalize_always_on_top(Some(1)));
    assert!(!normalize_always_on_top(Some(2)));
}
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run: `cargo test always_on_top -- --nocapture`

Expected: compilation FAIL because the field and helper do not exist.

- [ ] **Step 3: Add config and tray state**

Parse `always_on_top`, document it in `DEFAULT_CONFIG`, store normalized runtime state, and add a top-level `CheckMenuItem` labeled `始终置顶` to `Tray`. Pass the startup state into `build_tray`; clicking the item toggles state and updates the check. Hot reload updates both runtime state and check state.

- [ ] **Step 4: Add the 100 ms occlusion clock**

Store `last_occlusion_check` and cached `black_hole_occluded`. In `Event::AboutToWait`, when always-on-top is off and 100 ms elapsed, compute the visible radius as `current_hole_radius * primary_h * 7.5` and call `is_black_hole_occluded`. On API error set the cached value to `true`; when always-on-top is on force it to `false`.

```rust
let wanted = screensaver_wanted
    && (always_on_top || !black_hole_occluded);
```

Use this combined state in the existing Pane show/hide loop. The existing `overlay_visible` gate then disables cursor gravity and `DropWindow` whenever occluded.

- [ ] **Step 5: Connect the double-Ctrl controller**

Create `CtrlDoubleTapController` beside `CursorGravityController`. Each `AboutToWait`, call `take_triggered()`; on true, read `state.cursor_px()`, clamp it, assign both `state.center_px` and `state.pinned_px`, and let the existing tray-position synchronization mark “固定”. Remove all comments and menu labels referring to Ctrl+Shift; use `固定（双击 Ctrl 放置）`.

- [ ] **Step 6: Keep hidden polling responsive**

When the overlay is hidden because of occlusion, wake no later than the next 100 ms occlusion check. Preserve the 500 ms low-power interval for screen-saver-only hiding when no occlusion check is needed.

- [ ] **Step 7: Run integration tests and commit**

Run: `cargo test --all-targets`

Expected: all tests PASS.

```powershell
git add src/main.rs
git commit -m "feat: add desktop-aware visibility mode"
```

### Task 6: Update user documentation and verify the release build

**Files:**
- Modify: `README.md`
- Modify: `README.en.md`

- [ ] **Step 1: Update behavior documentation**

Document default desktop-aware visibility, opt-in always-on-top mode, permanent tray-only presence, double-Ctrl positioning, the `always_on_top` configuration key, and default 30 FPS in both languages. Remove Ctrl+Shift and default-60 claims.

- [ ] **Step 2: Run static verification**

Run: `cargo fmt --all -- --check`

Expected: exit code 0.

Run: `cargo test --all-targets`

Expected: exit code 0 and all tests PASS.

Run: `cargo check --all-targets`

Expected: exit code 0.

Run: `cargo build --release --bin BlackHoleTrash`

Expected: exit code 0 and `target/release/BlackHoleTrash.exe` exists.

- [ ] **Step 3: Perform Windows runtime verification**

Launch the release executable and verify its HWND extended style has `WS_EX_TOOLWINDOW` and not `WS_EX_APPWINDOW`. Verify no taskbar button, tray presence, uncovered visibility, overlap hiding/restoration, always-on-top bypass, disabled drag/cursor gravity while hidden, double-Ctrl placement without Ctrl+C/Ctrl+V false positives, and multi-monitor behavior.

- [ ] **Step 4: Commit documentation and any verification fixes**

```powershell
git add README.md README.en.md
git commit -m "docs: explain desktop visibility controls"
```
