# 30/60 FPS Options Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restrict tray and configuration FPS choices to 30 or 60, with 60 as the universal default and fallback.

**Architecture:** Keep the two menu values in `FPS_OPTS` and add one pure `normalize_fps` function shared by startup and hot reload. Initialize state at 60 and remove the uncapped frame-scheduling branch so runtime FPS can only be 30 or 60.

**Tech Stack:** Rust 2021, winit 0.29, tray-icon 0.19, inline Rust unit tests.

---

### Task 1: Enforce 30/60 FPS everywhere

**Files:**
- Modify: `src/main.rs:309-310`
- Modify: `src/main.rs:1520`
- Modify: `src/main.rs:1589`
- Modify: `src/main.rs:1761-1762`
- Modify: `src/main.rs:2234-2235`
- Modify: `src/main.rs:2298-2313`
- Test: `src/main.rs` inline `tests` module

- [ ] **Step 1: Write the failing FPS policy test**

```rust
#[test]
fn fps_policy_only_allows_thirty_or_sixty() {
    assert_eq!(FPS_OPTS, [("30 帧", 30), ("60 帧", 60)]);
    assert_eq!(normalize_fps(None), 60);
    assert_eq!(normalize_fps(Some(0)), 60);
    assert_eq!(normalize_fps(Some(30)), 30);
    assert_eq!(normalize_fps(Some(60)), 60);
    assert_eq!(normalize_fps(Some(120)), 60);
    assert_eq!(fps_option_index(30), 0);
    assert_eq!(fps_option_index(60), 1);
}
```

- [ ] **Step 2: Run the test and verify RED**

Run: `cargo test fps_policy_only_allows_thirty_or_sixty -- --nocapture`

Expected: compilation fails because `FPS_OPTS` still has three elements and `normalize_fps` does not exist.

- [ ] **Step 3: Implement the strict policy**

```rust
const DEFAULT_FPS: u32 = 60;
const FPS_OPTS: [(&str, u32); 2] = [("30 帧", 30), ("60 帧", 60)];

fn normalize_fps(value: Option<u32>) -> u32 {
    match value {
        Some(30) => 30,
        _ => DEFAULT_FPS,
    }
}

fn fps_option_index(fps: u32) -> usize {
    usize::from(fps != 30)
}
```

Initialize `State::fps` with `DEFAULT_FPS`; pass `normalize_fps(startup_cfg.fps)` during startup; call the same function during hot reload and update the tray check through `fps_option_index`. Add the current FPS to `build_tray`, use `fps_option_index` for its initial check, and pass `state.fps` when the tray is created. Replace the `fps == 0` polling branch with the existing finite `WaitUntil` scheduler for both supported values.

- [ ] **Step 4: Update generated config guidance**

Replace the default config text with:

```text
# Frame rate cap: 30 or 60. Other values fall back to 60.
#fps = 60
```

- [ ] **Step 5: Verify the focused test passes**

Run: `cargo test fps_policy_only_allows_thirty_or_sixty -- --nocapture`

Expected: one passing test with no warnings.

### Task 2: Verify and package

**Files:**
- Modify: none

- [ ] **Step 1: Run formatting and all tests**

Run: `cargo fmt --all -- --check`

Expected: exit code 0.

Run: `cargo test --all-targets`

Expected: all tests pass.

- [ ] **Step 2: Check all targets**

Run: `cargo check --all-targets`

Expected: exit code 0 without new warnings.

- [ ] **Step 3: Build Release**

Run: `cargo build --release --bin BlackHoleTrash`

Expected: `target\\release\\BlackHoleTrash.exe` builds successfully.
