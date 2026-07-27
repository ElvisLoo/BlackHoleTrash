# Visible Spin Animation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the existing spin levels produce obvious, continuous lens and accretion-disk rotation without phase jumps.

**Architecture:** Accumulate a signed `spin_phase` in `State`, using the current preset direction and a nonlinear speed curve. Send that phase through the existing uniform buffer and consume it in both lensing paths and the disk streak coordinates.

**Tech Stack:** Rust, wgpu uniform buffers, WGSL, Cargo tests

---

### Task 1: Continuous Spin Phase

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing phase tests**

Add tests that call `spin_angular_velocity` and `advance_spin_phase` and assert: zero spin does not move, `0.6 < 0.9 < 0.98`, negative direction decreases phase, and elapsed time is capped at `0.25` seconds.

- [ ] **Step 2: Verify the tests fail**

Run: `cargo test spin_phase -- --nocapture`

Expected: compilation fails because the phase helpers do not exist.

- [ ] **Step 3: Implement phase state**

Add:

```rust
fn spin_angular_velocity(spin: f32) -> f32 {
    0.9 * spin.clamp(0.0, 0.98).powi(3)
}

fn advance_spin_phase(phase: f32, spin: f32, direction: f32, elapsed: f32) -> f32 {
    phase
        + spin_angular_velocity(spin)
            * direction.signum()
            * elapsed.clamp(0.0, 0.25)
}
```

Store `spin_phase` and `last_spin_tick` in `State`, initialize them, advance them once per `AboutToWait`, and expose `spin_phase` in `Uniforms` while retaining one float of padding before the cursor vectors.

- [ ] **Step 4: Verify phase tests pass**

Run: `cargo test spin_phase -- --nocapture`

Expected: all spin phase tests pass.

### Task 2: Visible Shader Rotation

**Files:**
- Modify: `src/black_hole_trash.wgsl`
- Modify: `src/main.rs`

- [ ] **Step 1: Write a failing shader contract test**

Assert that the shader uniform contains `spin_phase`, `drag_twist` accepts it, both lens paths pass `u.spin_phase`, and disk streak coordinates use it.

- [ ] **Step 2: Verify the contract test fails**

Run: `cargo test shader_uses_continuous_spin_phase -- --nocapture`

Expected: assertion failure because WGSL has no `spin_phase` yet.

- [ ] **Step 3: Apply phase in WGSL**

Change the uniform tail to:

```wgsl
spin: f32,
spin_phase: f32,
_pad: f32,
```

Add the active phase to the radial `drag_twist`, pass it from both call sites, and subtract `u.spin_phase` from the disk streak swirl so lensing and material motion rotate in the same direction.

- [ ] **Step 4: Verify shader and project**

Run:

```text
cargo fmt --all -- --check
cargo test --all-targets
cargo run --example validate_shader
cargo clippy --all-targets -- -D warnings
cargo build --release
git diff --check
```

Expected: all commands exit successfully.

- [ ] **Step 5: Commit and launch**

Commit the implementation as `feat: animate visible black hole spin`, replace the running Release process with the newly built binary, and confirm it remains running.
