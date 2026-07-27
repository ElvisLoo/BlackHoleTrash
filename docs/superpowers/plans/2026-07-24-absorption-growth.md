# Absorption Growth Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add six animated black-hole growth tiers driven by successful session recycle counts, expose the same six tiers through the original tray size path, and localize the tray UI to Simplified Chinese.

**Architecture:** Define one `SIZE_TIERS` table and pure mapping/interpolation helpers in `src/main.rs`. `State` owns session progress plus a radius transition, and all consumers read its animated radius. Successful recycle events, tray choices, and hot-reloaded legacy sizes all update the same state so progress, target radius, and menu selection cannot diverge.

**Tech Stack:** Rust 2021, winit 0.29, tray-icon 0.19, Windows Shell recycle events, inline Rust unit tests.

---

### Task 1: Define and animate the six size tiers

**Files:**
- Modify: `src/main.rs:81-87`
- Modify: `src/main.rs:307-339`
- Modify: `src/main.rs:501-531`
- Modify: `src/main.rs:896-958`
- Test: `src/main.rs` inline `tests` module

- [ ] **Step 1: Write failing pure-function tests**

Add tests asserting `size_level_for_total` maps `0, 1..=2, 3..=4, 5..=9, 10..=19, 20+` to levels `0..=5`, `size_radius` returns `0.011 * [1.0, 1.25, 1.5, 1.75, 2.0, 4.0]`, `nearest_size_level` snaps old numeric config values, and `animated_radius` returns the start at `0.0`, a smooth midpoint at `0.2`, and the target at `0.4` seconds.

- [ ] **Step 2: Run tests and verify RED**

Run: `cargo test size_growth -- --nocapture`

Expected: compilation fails because the tier helpers do not exist.

- [ ] **Step 3: Add the shared tier model and pure helpers**

Create `SizeTier { label: &'static str, threshold: usize, multiplier: f32 }`, the six Chinese tier entries, `GROWTH_DURATION_SEC: f32 = 0.4`, and these functions:

```rust
fn size_level_for_total(total: usize) -> usize;
fn size_radius(level: usize) -> f32;
fn nearest_size_level(radius: f32) -> usize;
fn animated_radius(from: f32, to: f32, elapsed: f32) -> f32;
```

`animated_radius` clamps `elapsed / 0.4` to `0..=1` and applies `x * x * (3.0 - 2.0 * x)`.

- [ ] **Step 4: Store and render an active radius transition**

Replace the single `hole_radius` field with the current level, capped swallowed total, transition start/target radii, and `Instant`. Add `State::current_hole_radius()` and `State::set_size_level()`. Initialize at level zero without a startup animation. Use `current_hole_radius()` in uniforms and the Windows cursor-gravity field.

- [ ] **Step 5: Run focused tests and verify GREEN**

Run: `cargo test size_growth -- --nocapture`

Expected: all size-growth tests pass.

### Task 2: Synchronize successful recycling, manual selection, and config

**Files:**
- Modify: `src/main.rs:1623-1629`
- Modify: `src/main.rs:1782-1822`
- Modify: `src/main.rs:1985-2053`
- Modify: `src/main.rs:2064-2091`
- Test: `src/main.rs` inline `tests` module

- [ ] **Step 1: Write failing progress tests**

Add tests proving a successful count is saturating and capped at twenty, crossing several thresholds chooses only the final level, and manual levels correspond to progress values `0, 1, 3, 5, 10, 20`.

- [ ] **Step 2: Run tests and verify RED**

Run: `cargo test swallowed_progress -- --nocapture`

Expected: compilation fails because the progress helper does not exist.

- [ ] **Step 3: Implement and wire progress changes**

Add a pure `capped_swallowed_total(current, added)` helper and `State::absorb_successful_items(count)`. Call it only inside the current-generation `RecycleFinished` success branch. When the returned level changes, start one transition directly to that level and update the tray size check mark.

- [ ] **Step 4: Make manual and config changes use the same state path**

Tray selection calls `State::set_size_level(idx)`, synchronizing progress to the tier threshold. Startup and hot-reloaded `size` values call `nearest_size_level` and the same setter. If `size` is absent, level zero remains selected.

- [ ] **Step 5: Run focused and complete tests**

Run: `cargo test swallowed_progress -- --nocapture`

Expected: all progress tests pass.

Run: `cargo test --all-targets`

Expected: all existing and new tests pass.

### Task 3: Localize the tray to Simplified Chinese

**Files:**
- Modify: `src/main.rs:91-140`
- Modify: `src/main.rs:1395-1517`
- Modify: `src/main.rs:1720-1735`
- Modify: `src/main.rs:1969-1983`
- Test: `src/main.rs` inline `tests` module

- [ ] **Step 1: Write failing label tests**

Assert that all eight preset display names, six size labels, speed/FPS/screensaver/spin/position labels, and top-level tray titles contain no ASCII-only English UI text. Keep `PRESET_KEYS` unchanged.

- [ ] **Step 2: Run tests and verify RED**

Run: `cargo test tray_labels_are_chinese -- --nocapture`

Expected: the current English labels fail the assertions.

- [ ] **Step 3: Replace every visible tray string**

Translate preset display names, submenu titles/options, monitor aggregate label, open-config and quit items, update-available item, and tooltip. Preserve product name, numeric resolutions, `FPS`, and internal configuration keys where translation would break compatibility.

- [ ] **Step 4: Run label and full tests**

Run: `cargo test tray_labels_are_chinese -- --nocapture`

Expected: all localization tests pass.

Run: `cargo test --all-targets`

Expected: all tests pass.

### Task 4: Verify and package

**Files:**
- Modify: none

- [ ] **Step 1: Check formatting**

Run: `cargo fmt --all -- --check`

Expected: exit code 0.

- [ ] **Step 2: Check every target**

Run: `cargo check --all-targets`

Expected: exit code 0 without warnings introduced by this feature.

- [ ] **Step 3: Validate the shader**

Run: `cargo run --example validate_shader`

Expected: shader validation succeeds.

- [ ] **Step 4: Build the release executable**

Run: `cargo build --release --bin BlackHoleTrash`

Expected: `target\\release\\BlackHoleTrash.exe` is rebuilt successfully.

