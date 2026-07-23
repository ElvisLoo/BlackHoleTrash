# Black Hole Trash Repository Cleanup Design

## Scope

This change cleans the existing Singularity fork into a privately published
Black Hole Trash repository. It does not change the black-hole shader,
animation, drag behavior, or Recycle Bin implementation.

## Files to remove

The following legacy artifacts are not used by the Windows application build:

- `shaderglass/singularity.slang`
- `shaderglass/singularity.slangp`
- `docs/demo.gif`
- `examples/dx12_probe.rs`
- `examples/render_frame.rs`
- empty local `config/` and `watchdog/` directories

The WGSL validator, Windows capture path, D3D11-to-D3D12 sharing code,
screenshot repair code, platform integration, and macOS source are retained.

## Product rename

All active product-facing references are renamed to Black Hole Trash:

- README title and product prose
- Cargo package metadata and repository URL
- Windows title, executable metadata, user agent, single-instance names
- configuration filename and comments
- shader and icon asset filenames

The MIT license keeps the original GreenScreen410 copyright notice. The README
keeps a clear upstream attribution to `GreenScreen410/singularity`.

## GitHub publication

The authenticated account is `rrrjqy66`. A private repository named
`BlackHoleTrash` will be created under that account. The existing upstream
remote will be preserved as `upstream`; the new private repository will become
`origin`. The cleaned application will be committed on `main` and pushed to
the new origin.

## Validation

Before publication:

- scan for stale product-name references;
- run `cargo fmt`;
- run `cargo check --all-targets`;
- build `BlackHoleTrash.exe` in release mode;
- verify the removed files are not referenced;
- verify the new remote visibility is private.

No real user file will be deleted or recycled as part of validation.
