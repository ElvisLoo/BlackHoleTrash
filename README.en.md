# Black Hole Trash

[简体中文](README.md) | **English**

[![Latest Release](https://img.shields.io/github/v/release/rrrjqy66/BlackHoleTrash?label=release)](https://github.com/rrrjqy66/BlackHoleTrash/releases/latest)
[![Stars](https://img.shields.io/github/stars/rrrjqy66/BlackHoleTrash)](https://github.com/rrrjqy66/BlackHoleTrash/stargazers)
[![License](https://img.shields.io/github/license/rrrjqy66/BlackHoleTrash)](LICENSE)

Black Hole Trash is a compact, draggable Windows desktop black hole and a real Recycle Bin target. It bends the live desktop with real-time gravitational lensing and safely sends dropped files, folders, or multiple items to the Windows Recycle Bin instead of permanently deleting them.

## Download v1.2.0

[Download BlackHoleTrash-Setup-x64.exe](https://github.com/rrrjqy66/BlackHoleTrash/releases/download/v1.2.0/BlackHoleTrash-Setup-x64.exe) · [View the Release](https://github.com/rrrjqy66/BlackHoleTrash/releases/tag/v1.2.0)

The per-user installer requires no administrator privileges. It is currently unsigned, so Windows SmartScreen may display a warning. Use the `.sha256` asset on the same Release page to verify the download.

## Current Features

- **Real Recycle Bin drops**: Windows OLE `IDropTarget` accepts Explorer files, folders, and multi-item drops, then recycles them through `IFileOperation + FOFX_RECYCLEONDELETE`.
- **No permanent-delete fallback**: there is no `DeleteFile`, `RemoveDirectory`, or `std::fs::remove_*` fallback. Source items remain untouched if the Shell rejects or cancels an operation.
- **Real-time gravitational lensing**: Schwarzschild / Kerr geodesics, the event horizon, photon ring, relativistic accretion disk, and blackbody emission are rendered live.
- **Eight visual presets**: Inferno, Gargantua, Quasar, M87* donut, Blazar, Face-on ember, Pure lens, and Zen switch live from the tray with a smooth crossfade.
- **Cursor gravity and absorption trails**: the pointer meets progressive resistance, curves around the event horizon, and spirals inward with layered directional afterimages. A fast outward flick always escapes.
- **Six absorption growth tiers**: successful item counts at `0 / 1 / 3 / 5 / 10 / 20` select six sizes. The maximum is `4x` the starting size, with a smooth transition between tiers.
- **Chinese system tray**: visual preset, size, speed, FPS, screensaver, spin, position, monitor, config, and quit controls are available from the tray.
- **Fixed FPS choices**: the tray exposes only `30 FPS` and `60 FPS`. Missing, `0`, invalid, or unsupported config values fall back to `60`.
- **Live desktop capture**: Windows uses DX12, `windows-capture`, and a zero-copy D3D11 → D3D12 shared-texture path, with a CPU fallback.
- **Multi-monitor roaming**: each monitor owns an independent pane. The black hole can cross monitor boundaries or stay on one selected display.
- **Screensaver mode**: the hole appears after a configured idle period and disappears on the first keyboard or mouse input.
- **Capture exclusion and screenshot repair**: the overlay is excluded from its own desktop capture to prevent recursive mirror feedback; full-screen Print Screen images can composite the hole back into the clipboard image.
- **Update notices**: the app checks GitHub Releases at most once per day and adds a tray entry when a newer version is available. The check can be disabled in config.

## What's New in v1.2.0

- **Six size tiers**: a sixth tier unlocks after `20` swallowed items and reaches `4.00x` the starting size. Earlier tiers are `1.00 / 1.25 / 1.50 / 1.75 / 2.00x`.
- **Localized tray**: size, speed, FPS, screensaver, spin, position, monitor, config, and quit labels are now consistently Chinese.
- **Focused FPS policy**: only `30` and `60` remain, the uncapped rendering branch is removed, and the default and invalid-value fallback are both `60`.

## Usage

1. Launch `BlackHoleTrash.exe`.
2. Hold and drag the hole to place it. You can also hold `Ctrl+Shift` to pin it to the pointer.
3. Drag files, folders, or multiple items from Windows File Explorer.
4. Release them over the event horizon to send them to the Windows Recycle Bin.
5. Right-click the tray icon to change the visual preset, size, speed, FPS, position, and other settings.

The pointer absorption lasts about 160 ms and starts near the warm ring. It never traps the pointer: a fast outward flick escapes immediately. The native pointer stays hidden for at most 150 ms after absorption and returns on the next physical movement. Gravity pauses while repositioning the hole itself but remains active during Explorer file drags.

## Safety Rules

Black Hole Trash validates each path when items enter and again when they are released. It rejects locations that cannot be confirmed as safely recyclable, including:

- network paths and mapped network drives;
- removable devices and non-fixed disks;
- volume roots;
- items already inside `$Recycle.Bin`;
- Windows system directories;
- Black Hole Trash itself and its installation directory;
- missing, changed, or Shell-unresolvable paths.

If the Windows Shell rejects or cancels an operation, the files remain where they were. The app never switches to permanent deletion.

## System Requirements

- Windows 10 version 2004 or newer;
- Windows 11;
- an x64 desktop environment;
- a DirectX 12-capable GPU.

The release build does not require Rust, Node.js, Python, or another external runtime.

## Configuration

The config file is named `black-hole-trash.toml` and lives beside the executable. Changes reload automatically; common options are also available from the tray.

```toml
# FPS accepts only 30 or 60. Missing, invalid, and other values fall back to 30.
fps = 30

# Size snaps to six tiers: 0.011 / 0.01375 / 0.0165 / 0.01925 / 0.022 / 0.044
size = 0.011

# Look: inferno | gargantua | quasar | m87 | blazar | ember | lens | zen
preset = gargantua
```

Open the config file from the tray to see the complete generated template.

## Build from Source

Install [Rust](https://rustup.rs) with the MSVC toolchain.

```powershell
cargo build --release --bin BlackHoleTrash
cargo test --all-targets
cargo fmt --all -- --check
cargo check --all-targets
```

The executable is written to `target\release\BlackHoleTrash.exe`. The main WGSL shader is `src/black_hole_trash.wgsl`; Windows drag-and-recycle code lives under `src/platform/windows/`.

## Platform Support

- **Windows**: primary and tested platform, including desktop capture, Recycle Bin drops, tray controls, and installer packaging.
- **macOS**: ScreenCaptureKit and menu-bar structures remain in the source, but no real Mac test environment is available.
- **Linux**: not currently supported because X11 and Wayland do not provide a reliable overlay capture-exclusion path.

## Project Origins and License

Black Hole Trash is based on [GreenScreen410/singularity](https://github.com/GreenScreen410/singularity) and retains its MIT license and original attribution. The concept was also inspired by [ghostty-blackhole](https://github.com/s0xDk/ghostty-blackhole).

MIT. See [LICENSE](LICENSE).

## Release History

| Version | Features Added |
| --- | --- |
| **v1.2.0** | Six absorption growth tiers up to 4x, Chinese tray controls, 30/60-only FPS choices, and the invalid-FPS fallback policy. |
| **v1.1.0** | Cursor gravity, progressive resistance, spiral absorption, directional afterimages, outward-flick escape, and cross-monitor motion. |
| **v1.0.0** | First public Windows x64 installer with real-time lensing, Explorer recycle drops, multi-monitor support, and tray presets. |

## Star History

[![Star History Chart](assets/community/star-history.svg)](https://www.star-history.com/#rrrjqy66/BlackHoleTrash&Date)

## Community and Support

<table>
  <tr>
    <td align="center"><strong>Join the QQ Group</strong><br><img src="assets/community/qq-group.jpg" width="260" alt="QQ group QR code"><br>Group: 1083121107</td>
    <td align="center"><strong>Support the Project</strong><br><img src="assets/community/donate.jpg" width="260" alt="Donation QR code"><br>Thank you for your support</td>
  </tr>
</table>

Stars, issues, and QQ group discussions are welcome.
