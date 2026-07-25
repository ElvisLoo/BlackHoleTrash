# Visible Spin Animation Design

## Problem

The tray's spin setting currently changes the Kerr metric and a static lensing twist, but it does not produce continuous rotation. The label therefore promises motion that the renderer does not show clearly.

## Approaches

1. Integrate a continuous spin phase in application state and send it to the shader. This keeps phase continuous when the user changes levels and is the selected approach.
2. Derive phase directly from `time * spin`. This is smaller, but changing spin causes an immediate angular jump.
3. Animate only the disk texture. The disk already has orbital texture motion, so this would still leave the lensing background apparently static.

## Design

- Keep `spin` as the Kerr magnitude and add `spin_phase` as the accumulated visible rotation.
- Advance phase with a nonlinear speed curve so medium, high, and extreme settings are visibly distinct. Spin zero has zero angular velocity.
- Preserve the disk preset's rotation direction.
- Apply the phase to the radial frame-drag lens twist and to the disk streak coordinates.
- Keep phase continuous across tray and config changes; clamp unusually long frame gaps so resume-from-sleep does not jump.
- Keep the default at spin zero to preserve the current performance default.

## Verification

- Unit-test zero speed, increasing speed levels, direction, and phase continuity.
- Validate the WGSL shader and assert that both lens paths consume `spin_phase`.
- Run all targets, Clippy, release build, and launch the release binary.
