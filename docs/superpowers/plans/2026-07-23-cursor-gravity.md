# Cursor Gravity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Windows-only 20% mouse-resistance field and a smooth GPU-rendered cursor trail that spirals into the existing movable black-hole recycle target.

**Architecture:** A focused Windows module installs a `WH_MOUSE_LL` hook, shapes only movement events, distinguishes injected events, and publishes an allocation-free render snapshot. The existing per-pane uniform buffer carries a fixed trail history to WGSL, where a final composition function draws the proxy cursor without changing desktop capture or black-hole lensing.

**Tech Stack:** Rust 2021, windows 0.62 Win32 APIs, wgpu 23, WGSL, winit 0.29.

The implementation retains multi-monitor pane coordinates and does not modify
the D3D11-to-D3D12 zero-copy texture path or CPU capture fallback.

---

## File Structure

- Create `src/platform/windows/cursor_gravity.rs`: pure gravity math, Windows hook, cursor visibility guard, and fixed-size render snapshot.
- Modify `src/platform/windows/mod.rs`: export the controller and add widget-drag lifecycle events.
- Modify `src/platform/windows/drop_window.rs`: notify the controller when black-hole repositioning starts and stops.
- Modify `src/main.rs`: extend uniforms, own the controller, update its field, and translate global samples into each pane.
- Modify `src/black_hole_trash.wgsl`: declare cursor data and compose the continuous trail/proxy after existing lensing.
- Modify `src/screenshot_fix.rs`: force cursor proxy inactive in clipboard screenshots.
- Modify `README.md` and `README.en.md`: document cursor attraction, recovery, and Windows-only behavior.

### Task 1: Pure gravity model

**Files:**
- Create: `src/platform/windows/cursor_gravity.rs`
- Modify: `src/platform/windows/mod.rs`

- [ ] **Step 1: Define fixed data types and constants**

Add the following public surface, with the fixed 20% cap approved in the spec:

```rust
pub const TRAIL_SAMPLES: usize = 16;
const FIELD_RADIUS_MULTIPLIER: f64 = 4.2;
const MAX_RESISTANCE: f64 = 0.20;
const MAX_CORRECTION_PX: f64 = 7.0;
const ESCAPE_SPEED_PX: f64 = 34.0;
const HIDDEN_SECONDS: f32 = 0.150;

#[derive(Clone, Copy, Debug, Default)]
pub struct CursorSample {
    pub point: [f64; 2],
    pub age: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct CursorSnapshot {
    pub active: bool,
    pub point: [f64; 2],
    pub velocity: [f32; 2],
    pub strength: f32,
    pub absorption: f32,
    pub trail: [CursorSample; TRAIL_SAMPLES],
}

#[derive(Clone, Copy, Debug)]
struct Field {
    center: [f64; 2],
    radius_px: f64,
    enabled: bool,
    suspended: bool,
}
```

Implement `Default` manually for `CursorSnapshot` so every point, age, scalar,
and flag starts at zero.

- [ ] **Step 2: Implement the movement equation as a pure function**

Use a smooth cubic falloff, a 20% delta reduction, bounded inward/tangential
forces, and unconditional fast outward escape:

```rust
fn smoothstep01(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn shape_move(previous: [f64; 2], requested: [f64; 2], field: Field) -> ([f64; 2], f32) {
    let delta = [requested[0] - previous[0], requested[1] - previous[1]];
    let rel = [requested[0] - field.center[0], requested[1] - field.center[1]];
    let distance = rel[0].hypot(rel[1]);
    let field_radius = (field.radius_px * FIELD_RADIUS_MULTIPLIER).max(1.0);
    if !field.enabled || field.suspended || distance >= field_radius {
        return (requested, 0.0);
    }

    let direction = if distance > 0.001 {
        [rel[0] / distance, rel[1] / distance]
    } else {
        [0.0, 0.0]
    };
    let outward_speed = delta[0] * direction[0] + delta[1] * direction[1];
    if outward_speed >= ESCAPE_SPEED_PX {
        return (requested, 0.0);
    }

    let strength = smoothstep01(1.0 - distance / field_radius);
    let resistance = MAX_RESISTANCE * strength;
    let radial = (2.8 * strength).min(MAX_CORRECTION_PX);
    let tangent = 1.15 * strength;
    let correction = [
        -direction[0] * radial - direction[1] * tangent,
        -direction[1] * radial + direction[0] * tangent,
    ];
    (
        [
            previous[0] + delta[0] * (1.0 - resistance) + correction[0],
            previous[1] + delta[1] * (1.0 - resistance) + correction[1],
        ],
        strength as f32,
    )
}
```

- [ ] **Step 3: Add focused model tests without running them**

Add `#[cfg(test)]` cases in the new module covering:

```rust
#[test]
fn outside_field_is_unchanged() {
    let field = Field { center: [0.0, 0.0], radius_px: 20.0, enabled: true, suspended: false };
    assert_eq!(shape_move([200.0, 0.0], [210.0, 0.0], field).0, [210.0, 0.0]);
}

#[test]
fn resistance_never_exceeds_twenty_percent_before_force() {
    let field = Field { center: [0.0, 0.0], radius_px: 20.0, enabled: true, suspended: false };
    let (point, strength) = shape_move([30.0, 0.0], [40.0, 0.0], field);
    assert!(strength > 0.0);
    assert!(point[0] >= 30.0 + 10.0 * (1.0 - MAX_RESISTANCE) - MAX_CORRECTION_PX);
}

#[test]
fn fast_outward_flick_escapes() {
    let field = Field { center: [0.0, 0.0], radius_px: 20.0, enabled: true, suspended: false };
    assert_eq!(shape_move([10.0, 0.0], [60.0, 0.0], field), ([60.0, 0.0], 0.0));
}
```

- [ ] **Step 4: Expose the module and compile all targets**

Add `mod cursor_gravity;` and
`pub use cursor_gravity::{CursorGravityController, CursorSnapshot, TRAIL_SAMPLES};`
to `src/platform/windows/mod.rs`.

Run: `cargo check --all-targets`  
Expected: all library, binary, example, and test targets compile; no tests execute.

- [ ] **Step 5: Commit**

```powershell
git add src/platform/windows/cursor_gravity.rs src/platform/windows/mod.rs
git commit -m "Add cursor gravity model"
```

### Task 2: Windows input and cursor lifecycle

**Files:**
- Modify: `src/platform/windows/cursor_gravity.rs`

- [ ] **Step 1: Add the low-level hook controller**

Implement `CursorGravityController::new`, `set_field`, `set_suspended`,
`snapshot`, and `Drop`. Install `WH_MOUSE_LL` on the winit message-loop thread.
In the hook:

```rust
if n_code < 0 || w_param.0 as u32 != WM_MOUSEMOVE {
    return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
}
let event = unsafe { &*(l_param.0 as *const MSLLHOOKSTRUCT) };
if event.flags.0 & LLMHF_INJECTED.0 != 0 {
    return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
}
```

Call `shape_move`, publish the sample into the fixed ring buffer, call
`SetCursorPos` only when the rounded adjusted point differs from the requested
point, and return `LRESULT(1)` only after a successful replacement move.
Otherwise call `CallNextHookEx`.

- [ ] **Step 2: Add balanced visibility and absorption timing**

Use `GetCursorInfo` to observe visibility and `ShowCursor` only at state
transitions. Record whether this controller hid the cursor and balance that
single transition during `Drop`. Do not loop the display counter.

Advance `Inactive -> Attracting -> Absorbing -> Hidden` from distance and
`Instant`. `Hidden` lasts no longer than `HIDDEN_SECONDS`; the next physical
move restores visibility before further shaping.

- [ ] **Step 3: Keep the callback bounded**

Store the controller's shared state in `OnceLock<Arc<parking_lot::Mutex<_>>>`.
The callback performs only arithmetic, fixed-array writes, Win32 cursor calls,
and one non-blocking lock attempt. A failed `try_lock` passes the original event
through immediately.

- [ ] **Step 4: Compile**

Run: `cargo check --all-targets`  
Expected: successful Windows compile with no tests executed.

- [ ] **Step 5: Commit**

```powershell
git add src/platform/windows/cursor_gravity.rs
git commit -m "Add safe Windows cursor attraction"
```

### Task 3: Main-loop and movable-widget integration

**Files:**
- Modify: `src/platform/windows/mod.rs`
- Modify: `src/platform/windows/drop_window.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add widget-drag lifecycle events**

Extend `PlatformEvent` with `WidgetDragChanged(bool)`. Send `true` after
successful `SetCapture` on `WM_LBUTTONDOWN`; send `false` on `WM_LBUTTONUP`,
`WM_CAPTURECHANGED`, and `WM_CANCELMODE`.

- [ ] **Step 2: Own and update the controller**

Create `CursorGravityController` beside `DropWindow`. During `AboutToWait`,
compute the black-hole physical radius as
`state.hole_radius * state.primary_h` and call:

```rust
// State
#[cfg(windows)]
cursor_snapshot: platform::windows::CursorSnapshot,

// AboutToWait
cursor_gravity.set_field(
    state.center_px,
    state.hole_radius as f64 * state.primary_h,
    overlay_visible && state.pinned_px.is_some(),
);
state.cursor_snapshot = cursor_gravity.snapshot();
```

Map `WidgetDragChanged(active)` to `cursor_gravity.set_suspended(active)`.
Read one `CursorSnapshot` into `State` before pane redraw requests.

- [ ] **Step 3: Preserve OLE file dragging**

Do not suspend on `DragEntered`, `DragMoved`, or `DragLeft`. The native adjusted
position must continue through the existing OLE drop window so file drops still
reach `IFileOperation`.

- [ ] **Step 4: Compile and commit**

Run: `cargo check --all-targets`  
Expected: successful compile.

```powershell
git add src/platform/windows/mod.rs src/platform/windows/drop_window.rs src/main.rs
git commit -m "Connect cursor gravity to the recycle target"
```

### Task 4: Per-pane cursor uniforms

**Files:**
- Modify: `src/main.rs`
- Modify: `src/screenshot_fix.rs`
- Modify: `src/black_hole_trash.wgsl`

- [ ] **Step 1: Extend the Rust and WGSL uniform blocks identically**

Append aligned fields to Rust `Uniforms`:

```rust
pub cursor: [f32; 4],        // pane-local x/y, strength, absorption
pub cursor_motion: [f32; 4], // pane-local velocity x/y, active, unused
pub cursor_trail: [[f32; 4]; TRAIL_SAMPLES], // x/y, age, valid
```

Append matching WGSL fields:

```wgsl
cursor: vec4<f32>,
cursor_motion: vec4<f32>,
cursor_trail: array<vec4<f32>, 16>,
```

- [ ] **Step 2: Convert global history for every pane**

In `pane_uniforms`, convert each snapshot position with the same monitor origin
and dimensions already used for `center`. Scale velocity by pane dimensions.
Points may remain outside `[0, 1]` so a cross-monitor trail stays continuous.

- [ ] **Step 3: Exclude the proxy from clipboard screenshots**

In `screenshot_fix.rs`, after copying `shot.uniforms`, set:

```rust
u.cursor_motion[2] = 0.0;
u.cursor[2] = 0.0;
```

- [ ] **Step 4: Compile and commit**

Run: `cargo check --all-targets`  
Expected: successful compile with matching uniform sizes.

```powershell
git add src/main.rs src/screenshot_fix.rs src/black_hole_trash.wgsl
git commit -m "Pass cursor trail data to each pane"
```

### Task 5: Smooth WGSL proxy and trail

**Files:**
- Modify: `src/black_hole_trash.wgsl`

- [ ] **Step 1: Add analytic distance helpers**

Add a point-to-segment distance function and a `cursor_overlay` function. Loop
over the 15 fixed segments, taper width from 1 to 8 pixels using sample age,
and accumulate a soft glow with `exp2(-distance * distance / width)`.

- [ ] **Step 2: Draw an oriented proxy head**

Build tangent/normal axes from `u.cursor_motion.xy`. Render the arrow with
signed distances for a triangular head and short stem. Scale along the tangent
by `mix(1.0, 2.1, u.cursor.z)` and shrink both axes by
`1.0 - smoothstep(0.72, 1.0, u.cursor.w)`.

- [ ] **Step 3: Compose without rewriting lensing**

Wrap both existing color exits:

```wgsl
return vec4<f32>(cursor_overlay(col + stars(d) * u.star * window, uv), 1.0);
```

and:

```wgsl
return vec4<f32>(cursor_overlay(col, uv), 1.0);
```

The helper returns the input color unchanged when `u.cursor_motion.z < 0.5`.

- [ ] **Step 4: Validate WGSL and commit**

Run: `cargo run --example validate_shader`  
Expected: `WGSL OK - parsed and validated.`

```powershell
git add src/black_hole_trash.wgsl
git commit -m "Render smooth cursor absorption trail"
```

### Task 6: Documentation and requested verification

**Files:**
- Modify: `README.md`
- Modify: `README.en.md`

- [ ] **Step 1: Document behavior and recovery**

Add a Chinese-first feature note and matching English note describing the
Windows-only 20% maximum resistance, fast-flick escape, 150 ms absorption, and
automatic native-cursor restoration.

- [ ] **Step 2: Format and check**

Run:

```powershell
cargo fmt --all
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo run --example validate_shader
cargo build --release
```

Expected: every command exits with code 0; WGSL reports validation success;
`target\release\BlackHoleTrash.exe` is produced. Do not run `cargo test`.

- [ ] **Step 3: Inspect the final diff**

Run: `git diff --check`  
Expected: no whitespace errors.

Run: `git status --short`  
Expected: only the intended source, shader, README, and plan changes.

- [ ] **Step 4: Commit**

```powershell
git add README.md README.en.md src docs/superpowers/plans/2026-07-23-cursor-gravity.md
git commit -m "Add smooth cursor gravity effect"
```
