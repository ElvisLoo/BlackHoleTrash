# Black Hole Trash Windows Installer Design

## Goal

Produce a public, downloadable Windows x64 installer for Black Hole Trash and
publish it as GitHub Release `v1.0.0` with a SHA-256 checksum file.

## Installer format

Use Inno Setup 6 to generate:

```text
BlackHoleTrash-Setup-x64.exe
```

The installer is per-user and does not request administrator privileges. It
installs into:

```text
%LocalAppData%\Programs\BlackHoleTrash
```

Installed files:

- `BlackHoleTrash.exe`
- `README.md`
- `README.en.md`
- `LICENSE`

The installer creates a Start Menu shortcut, offers an optional desktop
shortcut, registers a standard Windows uninstall entry, and offers to launch
Black Hole Trash when installation completes.

## Uninstall behavior

Uninstall removes only files and shortcuts created by this installer. It does
not inspect, empty, restore, or otherwise modify the Windows Recycle Bin. User
configuration outside the installation directory is not deleted.

## Build integration

Add:

- `installer/BlackHoleTrash.iss` for the Inno Setup definition;
- `scripts/build-installer.ps1` to build the Rust release executable, compile
  the installer, and write the SHA-256 checksum;
- `dist/BlackHoleTrash-Setup-x64.exe`;
- `dist/BlackHoleTrash-Setup-x64.exe.sha256`.

Inno Setup is a build-time dependency only and is not required on end-user
machines.

## Public GitHub release

The repository remains public. Create tag and release `v1.0.0` in
`rrrjqy66/BlackHoleTrash`, then upload:

- `BlackHoleTrash-Setup-x64.exe`;
- `BlackHoleTrash-Setup-x64.exe.sha256`.

Release notes are Chinese-first and state that the installer is unsigned, so
Windows SmartScreen may display a warning.

## Validation

- run `cargo fmt --all`;
- run `cargo check --all-targets`;
- build `BlackHoleTrash.exe` in release mode;
- compile the Inno Setup script without errors;
- verify the installer and checksum filenames;
- independently recompute SHA-256 and compare it with the checksum file;
- verify GitHub repository visibility is public;
- verify Release `v1.0.0` is public and both assets are downloadable.

No real files are recycled or permanently deleted during validation. The
installer is not code-signed.
