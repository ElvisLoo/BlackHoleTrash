# Absorption Jet Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render a 0.90-second bipolar energy jet and expanding shockwave whenever Windows confirms a successful recycle batch.

**Architecture:** Keep the existing single-pass `wgpu` pipeline. A small CPU-side `AbsorptionJet` value owns event timing and batch energy, one new `vec4` uniform carries its frame state to every pane, and a procedural WGSL overlay composites the jet before the existing cursor overlay.

**Tech Stack:** Rust 2021, `wgpu` 23, WGSL, `bytemuck`, naga shader validation, built-in Rust tests.

---

## File Map

- Modify `src/main.rs`: define the CPU pulse lifecycle, add the uniform slot, trigger pulses from successful recycle results, and add unit tests.
- Modify `src/black_hole_trash.wgsl`: mirror the uniform slot and render the main jet, counterjet, flash, and shockwave in both near-field and far-field output paths.
- Modify `examples/validate_shader.rs`: no source change expected; run it as the existing offline WGSL parser and validator.

### Task 1: Absorption Jet Lifecycle

**Files:**
- Modify: `src/main.rs`
- Test: `src/main.rs` (`#[cfg(test)] mod tests`)

- [ ] **Step 1: Write failing lifecycle and energy tests**

Add these tests beside the existing absorption-growth tests:

```rust
#[test]
fn absorption_jet_energy_scales_logarithmically_and_caps_at_twenty() {
    assert_eq!(AbsorptionJet::at(0, std::time::Instant::now()), None);
    assert_close(
        AbsorptionJet::at(1, std::time::Instant::now())
            .expect("one item should trigger a jet")
            .energy,
        1.0,
    );
    let two = AbsorptionJet::at(2, std::time::Instant::now())
        .expect("two items should trigger a jet")
        .energy;
    assert!(two > 1.0 && two < 1.5);
    assert_close(
        AbsorptionJet::at(20, std::time::Instant::now())
            .expect("twenty items should trigger a jet")
            .energy,
        1.5,
    );
    assert_close(
        AbsorptionJet::at(usize::MAX, std::time::Instant::now())
            .expect("large batches should still trigger a jet")
            .energy,
        1.5,
    );
}

#[test]
fn absorption_jet_uniforms_cover_start_midpoint_and_completion() {
    let started_at = std::time::Instant::now();
    let jet = AbsorptionJet::at(1, started_at).expect("jet");
    assert_eq!(jet.uniforms_at(started_at), [0.0, 1.0, 1.0, 0.0]);
    assert_eq!(
        jet.uniforms_at(started_at + std::time::Duration::from_millis(450)),
        [0.5, 1.0, 1.0, 0.0],
    );
    assert_eq!(
        jet.uniforms_at(started_at + std::time::Duration::from_millis(900)),
        [1.0, 1.0, 0.0, 0.0],
    );
    assert_eq!(
        jet.uniforms_at(started_at + std::time::Duration::from_secs(2)),
        [1.0, 1.0, 0.0, 0.0],
    );
}
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run:

```powershell
cargo test absorption_jet -- --nocapture
```

Expected: compilation fails because `AbsorptionJet` does not exist yet.

- [ ] **Step 3: Implement the minimal CPU pulse value**

Add near the existing size-growth helpers:

```rust
const ABSORPTION_JET_DURATION_SEC: f32 = 0.9;

#[derive(Clone, Copy, Debug, PartialEq)]
struct AbsorptionJet {
    started_at: std::time::Instant,
    energy: f32,
}

impl AbsorptionJet {
    fn at(count: usize, started_at: std::time::Instant) -> Option<Self> {
        if count == 0 {
            return None;
        }
        let capped = count.min(20) as f32;
        let energy = 1.0 + 0.5 * capped.ln() / 20.0_f32.ln();
        Some(Self { started_at, energy })
    }

    fn new(count: usize) -> Option<Self> {
        Self::at(count, std::time::Instant::now())
    }

    fn uniforms_at(self, now: std::time::Instant) -> [f32; 4] {
        let elapsed = now.saturating_duration_since(self.started_at).as_secs_f32();
        let progress = (elapsed / ABSORPTION_JET_DURATION_SEC).clamp(0.0, 1.0);
        let active = if elapsed < ABSORPTION_JET_DURATION_SEC {
            1.0
        } else {
            0.0
        };
        [progress, self.energy, active, 0.0]
    }
}
```

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run:

```powershell
cargo test absorption_jet -- --nocapture
```

Expected: both `absorption_jet_*` tests pass.

- [ ] **Step 5: Commit the lifecycle**

```powershell
git add src/main.rs
git commit -m "feat: add absorption jet lifecycle"
```

### Task 2: Event and Uniform Wiring

**Files:**
- Modify: `src/main.rs`
- Test: `src/main.rs` (`#[cfg(test)] mod tests`)

- [ ] **Step 1: Write a failing uniform contract test**

Add a test that describes the required CPU-side payload:

```rust
#[test]
fn absorption_jet_uniform_payload_is_one_aligned_vec4() {
    let started_at = std::time::Instant::now();
    let jet = AbsorptionJet::at(3, started_at).expect("jet");
    let payload = jet.uniforms_at(started_at);
    assert_eq!(payload.len(), 4);
    assert_eq!(payload[2], 1.0);
    assert_eq!(std::mem::size_of_val(&payload), 16);
    let uniforms = Uniforms {
        resolution: [1.0, 1.0],
        time: 0.0,
        has_desktop: 0.0,
        look: [0.0; 14],
        hole_radius: 0.01,
        center: [0.5, 0.5],
        spin: 0.0,
        _pad: [0.0; 2],
        cursor: [0.0; 4],
        cursor_motion: [0.0; 4],
        absorption_jet: payload,
        cursor_trail: [[0.0; 4]; CURSOR_TRAIL_SAMPLES],
    };
    assert_eq!(uniforms.absorption_jet, payload);
}
```

- [ ] **Step 2: Run the contract test and verify RED**

Run:

```powershell
cargo test absorption_jet_uniform_payload_is_one_aligned_vec4 -- --nocapture
```

Expected: compilation fails because `Uniforms::absorption_jet` is missing.

- [ ] **Step 3: Add the uniform field and State ownership**

Add this field to `Uniforms` immediately after `cursor_motion`:

```rust
pub absorption_jet: [f32; 4], // progress, batch energy, active, reserved
```

Add this field to `State`:

```rust
absorption_jet: Option<AbsorptionJet>,
```

Initialize it to `None` in `State::new`. Add these methods:

```rust
fn trigger_absorption_jet(&mut self, count: usize) {
    self.absorption_jet = AbsorptionJet::new(count);
}

fn current_absorption_jet_uniforms(&self) -> [f32; 4] {
    self.absorption_jet
        .map(|jet| jet.uniforms_at(std::time::Instant::now()))
        .unwrap_or([0.0; 4])
}
```

Set `absorption_jet: self.current_absorption_jet_uniforms()` when constructing each Pane's `Uniforms` value.

- [ ] **Step 4: Trigger the pulse only from the current successful recycle result**

In the existing `PlatformEvent::RecycleFinished(result) if result.generation == active_generation` success branch, call this before updating growth:

```rust
state.trigger_absorption_jet(result.paths.len());
```

Do not add calls to the failure, stale-generation, manual size, or drag-preview paths.

- [ ] **Step 5: Run tests and verify GREEN**

Run:

```powershell
cargo test absorption_jet -- --nocapture
cargo test
```

Expected: the focused tests and complete test suite pass.

- [ ] **Step 6: Commit event and uniform wiring**

```powershell
git add src/main.rs
git commit -m "feat: trigger absorption jet after recycling"
```

### Task 3: Procedural WGSL Jet and Shockwave

**Files:**
- Modify: `src/main.rs`
- Modify: `src/black_hole_trash.wgsl`
- Test: `src/main.rs` (`#[cfg(test)] mod tests`)

- [ ] **Step 1: Write a failing shader integration test**

Add this source-contract test:

```rust
#[test]
fn shader_composites_absorption_jet_in_both_output_paths() {
    let shader = include_str!("black_hole_trash.wgsl");
    assert!(shader.contains("absorption_jet: vec4<f32>"));
    assert!(shader.contains("fn absorption_jet_overlay("));
    assert_eq!(shader.matches("absorption_jet_overlay(").count(), 3);
}
```

- [ ] **Step 2: Run the shader contract test and verify RED**

Run:

```powershell
cargo test shader_composites_absorption_jet_in_both_output_paths -- --nocapture
```

Expected: the test fails because the WGSL uniform and overlay function are absent.

- [ ] **Step 3: Mirror the uniform field in WGSL**

Insert this field after `cursor_motion` so the Rust and WGSL layouts stay identical:

```wgsl
absorption_jet: vec4<f32>, // progress, batch energy, active, reserved
```

- [ ] **Step 4: Implement the procedural overlay**

Add this helper before `cursor_overlay`:

```wgsl
fn absorption_jet_overlay(base: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    if (u.absorption_jet.z < 0.5) {
        return base;
    }

    let progress = clamp(u.absorption_jet.x, 0.0, 1.0);
    let energy = clamp(u.absorption_jet.y, 1.0, 1.5);
    let energy01 = (energy - 1.0) / 0.5;
    let aspect = u.resolution.x / max(u.resolution.y, 1.0);
    let center = vec2<f32>(u.center_x, u.center_y);
    let rel = (uv - center) * vec2<f32>(aspect, 1.0);
    let radius = max(u.hole_radius, 1e-4);

    var axis = normalize(rot(vec2<f32>(0.0, -1.0), -u.roll));
    if (axis.y > 0.0) {
        axis = -axis;
    }
    let tangent = vec2<f32>(-axis.y, axis.x);
    let axial = dot(rel, axis);
    let transverse = dot(rel, tangent);

    let attack = smoothstep(0.0, 0.13, progress);
    let decay = 1.0 - smoothstep(0.45, 1.0, progress);
    let envelope = attack * decay;
    let extension = smoothstep(0.0, 0.24, progress);
    let main_length = min(radius * mix(11.0, 14.0, energy01), 0.52) * extension;
    let base_width = radius * mix(0.38, 0.50, energy01);

    let main_distance = max(axial, 0.0);
    let main_fraction = clamp(main_distance / max(main_length, radius), 0.0, 1.0);
    let main_cap = step(0.0, axial)
        * (1.0 - smoothstep(main_length * 0.82, max(main_length, radius), main_distance));
    let main_width = base_width * mix(1.55, 0.42, main_fraction);
    let filament = 0.84 + 0.16 * sin(
        main_distance / radius * 10.0 - u.time * 34.0 + transverse / radius * 2.4,
    );
    let main_core = exp2(-pow(transverse / max(main_width * 0.20, 1e-5), 2.0) * 2.2)
        * main_cap
        * filament;
    let main_halo = exp2(-pow(transverse / max(main_width, 1e-5), 2.0) * 1.5)
        * main_cap;

    let counter_distance = max(-axial, 0.0);
    let counter_length = main_length * 0.64;
    let counter_fraction = clamp(
        counter_distance / max(counter_length, radius),
        0.0,
        1.0,
    );
    let counter_cap = step(0.0, -axial)
        * (1.0 - smoothstep(
            counter_length * 0.78,
            max(counter_length, radius),
            counter_distance,
        ));
    let counter_width = base_width * mix(1.35, 0.50, counter_fraction);
    let counter = exp2(-pow(transverse / max(counter_width, 1e-5), 2.0) * 1.7)
        * counter_cap
        * 0.18;

    let radial = length(rel);
    let shock_progress = smoothstep(0.02, 0.72, progress);
    let shock_radius = radius * mix(1.15, mix(4.8, 5.7, energy01), shock_progress);
    let shock_width = radius * mix(0.30, 0.09, shock_progress);
    let shock = exp2(-pow((radial - shock_radius) / max(shock_width, 1e-5), 2.0) * 2.8)
        * (1.0 - smoothstep(0.18, 0.78, progress));
    let flash = exp2(-pow(radial / max(radius * 0.95, 1e-5), 2.0) * 2.4)
        * (1.0 - smoothstep(0.0, 0.28, progress));

    var light = vec3<f32>(0.0);
    light = light
        + vec3<f32>(0.48, 0.60, 1.0) * main_halo * envelope * energy
        + vec3<f32>(0.96, 0.98, 1.0) * main_core * envelope * energy;
    light = light
        + vec3<f32>(0.58, 0.48, 1.0) * counter * envelope * energy
        + vec3<f32>(0.68, 0.80, 1.0) * shock * energy
        + vec3<f32>(0.92, 0.95, 1.0) * flash * energy;
    let contribution = vec3<f32>(1.0) - exp(-min(light, vec3<f32>(4.0)));
    return min(base + contribution, vec3<f32>(1.25));
}
```

Call it in both fragment output paths before `cursor_overlay`:

```wgsl
return vec4<f32>(cursor_overlay(absorption_jet_overlay(col, uv), uv), 1.0);
```

For the far-field path, pass its existing `col + stars(...)` expression as the first argument instead of `col`.

- [ ] **Step 5: Run the contract test and offline WGSL validation**

Run:

```powershell
cargo test shader_composites_absorption_jet_in_both_output_paths -- --nocapture
cargo run --example validate_shader
```

Expected: the contract test passes and validation prints `WGSL OK - parsed and validated.`

- [ ] **Step 6: Run the full verification suite**

Run:

```powershell
cargo fmt -- --check
cargo test
cargo check --all-targets
cargo build --release
git diff --check
```

Expected: all commands exit successfully with no Rust, WGSL, or whitespace errors.

- [ ] **Step 7: Commit the shader effect**

```powershell
git add src/main.rs src/black_hole_trash.wgsl
git commit -m "feat: render jet when black hole absorbs files"
```

### Task 4: Native Visual Smoke Test

**Files:**
- No source changes expected

- [ ] **Step 1: Launch the release executable**

Run:

```powershell
Start-Process -FilePath 'target\release\BlackHoleTrash.exe'
```

Expected: the desktop overlay starts and the black hole remains correctly positioned.

- [ ] **Step 2: Recycle one disposable test file through the black hole**

Create and open a dedicated temporary directory:

```powershell
$jetSmokeDir = Join-Path $env:TEMP 'BlackHoleTrash-jet-smoke'
New-Item -ItemType Directory -Force -Path $jetSmokeDir | Out-Null
New-Item -ItemType File -Force -Path (Join-Path $jetSmokeDir 'single-smoke-test.txt') | Out-Null
Start-Process -FilePath explorer.exe -ArgumentList $jetSmokeDir
```

Drag `single-smoke-test.txt` from Explorer into the black hole. Confirm exactly one 0.90-second pulse appears only after Shell accepts the recycle operation.

- [ ] **Step 3: Check the selected visual contract**

Confirm the main jet is bright and dominant, the counterjet is faint, the shockwave expands from the center, the direction stays perpendicular to the rendered disk, and the effect leaves no residual pixels after completion.

- [ ] **Step 4: Check multi-item scaling**

Create five disposable top-level files:

```powershell
1..5 | ForEach-Object {
    New-Item -ItemType File -Force -Path (Join-Path $jetSmokeDir "batch-smoke-$_.txt") | Out-Null
}
```

Select all five files and drag them into the black hole together. Confirm there is still one pulse, with mildly increased width and brightness rather than repeated firing.

- [ ] **Step 5: Record the verification result**

Run:

```powershell
git status --short
git log -4 --oneline
```

Expected: source changes are committed, `.superpowers/` remains ignored, and the lifecycle, event wiring, and shader commits follow the design and plan commits.
