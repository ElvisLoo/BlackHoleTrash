# Cursor Gravity and Visual Absorption Design

Date: 2026-07-23  
Status: Approved for implementation

## Goal

Add a Windows-only mouse gravity effect to Black Hole Trash. When the pointer
passes near the black hole, it should become slightly harder to move, leave a
smooth continuous trail, curve around the hole, and visually collapse into the
event horizon. The effect must preserve normal clicking, file dragging, OLE
drop behavior, multi-monitor movement, and emergency cursor recovery.

This feature extends the current product. It does not replace or rewrite the
existing Schwarzschild/Kerr shader, Windows desktop capture, D3D11-to-D3D12
shared-texture path, CPU capture fallback, pane architecture, capture
exclusion, recycle-bin operation, tray, or presets.

## Approved Experience

- Gravity starts outside the visible disk and increases continuously with
  proximity.
- Pointer resistance is capped at 20%. There is no hard cursor lock.
- A small tangential component bends the path into a smooth orbital arc.
- A fast outward movement always escapes the field.
- Inside the visual field, a GPU proxy replaces the native arrow.
- The proxy becomes a continuous blurred ribbon rather than a series of copied
  arrow shapes.
- Stretch, blur, trail length, and angular curvature increase gradually toward
  the event horizon.
- On absorption, the proxy shrinks into the centre and disappears for 150 ms.
- The next physical mouse movement restores the pointer immediately.
- The same behavior remains active during Shell file drags so the pointer can
  be guided naturally onto the recycle drop target.

The motion reference is the user-provided video
`954ba9888313205a18250c4fbd567b1d.mp4`. Only its qualitative motion is used:
continuous inertial curvature, soft trailing, and smooth convergence. The
video is not copied into the repository or product.

## Architecture

### 1. Windows cursor-gravity controller

Create `src/platform/windows/cursor_gravity.rs`. It owns the Windows low-level
mouse hook, native cursor visibility guard, injected-event guard, and the
latest gravity sample shared with the render loop.

The controller receives the current global black-hole centre and physical
radius from `State`. It publishes:

- actual global cursor position;
- GPU proxy position;
- normalized distance and gravity strength;
- velocity and tangent direction;
- absorption phase;
- a bounded history of recent proxy samples.

The controller is Windows-only. Other platforms keep their current behavior
and compile without cursor-gravity fields or APIs.

### 2. Movement shaping

For each non-injected mouse move:

1. Compute the physical delta from the previous input position.
2. Compute normalized distance from the current black-hole centre.
3. Apply a smooth radial falloff that is zero outside the field.
4. Reduce the input delta by at most 20%.
5. Add a bounded inward force and a smaller spin-aligned tangential force.
6. Clamp the correction so one update cannot jump or trap the cursor.
7. Submit the adjusted position as one injected move and ignore that injected
   event when it returns through the hook.

The outward component of a fast movement reduces gravity strength immediately.
This makes a quick flick an unconditional escape gesture. Mouse buttons,
wheel input, keyboard input, and non-movement events pass through unchanged.

The movement function is a pure Rust calculation with explicit constants for
field radius, maximum resistance, radial attraction, tangential attraction,
maximum correction, and escape velocity. The Windows hook only adapts between
native events and this calculation.

### 3. Cursor visibility

The native pointer is hidden only while the visual proxy is active. A
balanced RAII visibility guard records every visibility transition and
restores the cursor:

- when the pointer leaves the gravity field;
- after an absorption cycle when new movement begins;
- when the application exits normally;
- when the hook is removed;
- when cursor replacement or hook setup fails.

If safe native cursor hiding cannot be established, the controller disables
the proxy and movement shaping together. It must never show both a native and
proxy pointer or leave Windows without a visible pointer.

### 4. Render data

Extend the existing uniform data without changing the desktop texture binding:

- per-pane proxy position;
- velocity/tangent;
- gravity strength;
- absorption progress;
- active flag.

Add one small storage or uniform buffer for the bounded trail history. Each
global sample is converted to pane-local coordinates using the pane's existing
virtual-desktop origin and dimensions. This preserves seamless behavior across
monitor boundaries.

The screenshot compositor sets cursor gravity inactive. Screenshots therefore
retain the existing black-hole render and never capture a synthetic cursor.

### 5. Shader composition

Keep the existing lensing and accretion-disk calculations intact. After the
desktop and black-hole color are resolved, add a cursor-proxy composition
stage:

- reconstruct a smooth curve from recent cursor samples;
- render a tapered, continuous trail with analytic distance fields;
- draw the arrow head only while the proxy is outside the final absorption
  phase;
- stretch the proxy along velocity and curve it along the gravity tangent;
- increase softness and reduce opacity toward the event horizon;
- shrink the complete proxy to zero during the final absorption phase.

The trail is an emissive overlay with bounded intensity. It does not feed back
into desktop capture and does not alter the zero-copy texture path.

## Interaction With Existing Recycle Drop

The existing 88-pixel OLE drop window remains the authoritative drop target
and continues to track the black-hole centre. The adjusted native cursor
position, not only the rendered proxy, enters this window, so `IDropTarget`
receives normal `DragEnter`, `DragOver`, and `Drop` calls.

The gravity controller does not inspect or retain file paths. Recycling remains
exclusively in the current `IFileOperation` path, which sends files to the
Recycle Bin rather than permanently deleting them.

Dragging the black-hole widget itself temporarily suspends cursor gravity.
This prevents the movable recycle target from attracting the pointer while the
user is repositioning it.

## Timing and State

The cursor effect has four states:

1. `Inactive`: native cursor visible and no movement shaping.
2. `Attracting`: movement shaping and GPU proxy are active.
3. `Absorbing`: proxy curves, stretches, and collapses into the centre.
4. `Hidden`: proxy and native cursor are hidden for at most 150 ms.

Any new physical movement from `Hidden` goes directly to `Attracting` or
`Inactive` based on current distance. Loss of hook state, application shutdown,
or an invalid monitor coordinate transitions immediately to `Inactive` and
restores the native cursor.

## Performance Boundaries

- Cursor history is fixed-size and allocation-free during movement.
- The hook callback performs no logging, file access, GPU work, or blocking
  channel operation.
- The render loop reads one snapshot and uploads bounded data once per frame.
- Shader trail work is skipped when inactive.
- No new desktop copy or readback is introduced.

## Verification

Implementation verification follows the user's request not to run the full
test suite:

- `cargo fmt --all`;
- `cargo check --all-targets`;
- `cargo clippy --all-targets`;
- the existing WGSL validation example;
- a Windows release build.

The pure movement calculation will include focused unit-test cases in source,
but `cargo test` will not be run during this implementation unless the user
changes that instruction. Manual acceptance covers:

- normal movement outside the field;
- gradual 20% maximum resistance;
- fast-flick escape;
- smooth trail and orbital convergence;
- 150 ms recovery after absorption;
- dragging and recycling a file;
- dragging the black-hole widget;
- crossing monitor boundaries;
- normal exit and forced hook teardown with native cursor restored.

## Out of Scope

- Permanent changes to Windows system cursor files or cursor scheme;
- accessibility settings UI for this iteration;
- configurable gravity percentage;
- non-Windows cursor attraction;
- changes to the installer or the published v1.0.0 release.
