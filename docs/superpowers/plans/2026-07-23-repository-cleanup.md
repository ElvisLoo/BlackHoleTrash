# Black Hole Trash Repository Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove obsolete Singularity artifacts, rename the active product to Black Hole Trash, and publish the cleaned Windows application to a new private GitHub repository.

**Architecture:** Preserve the existing renderer, capture pipeline, drag target, and Recycle Bin implementation. Remove only unreferenced legacy artifacts, update active identifiers and asset filenames in place, validate the Windows build, then preserve the upstream remote while publishing the cleaned `main` branch to `rrrjqy66/BlackHoleTrash`.

**Tech Stack:** Rust 2021, wgpu 23, winit 0.29, Windows OLE/Shell APIs, Git, GitHub CLI.

---

### Task 1: Remove obsolete legacy artifacts

**Files:**
- Delete: `shaderglass/singularity.slang`
- Delete: `shaderglass/singularity.slangp`
- Delete: `docs/demo.gif`
- Delete: `examples/dx12_probe.rs`
- Delete: `examples/render_frame.rs`
- Keep: `examples/validate_shader.rs`

- [x] **Step 1: Verify every deletion target is outside the active application**

Run:

```powershell
rg -n "shaderglass|demo.gif|dx12_probe|render_frame" Cargo.toml build.rs src README.md
```

Expected: only README documentation references; no `src/` or Cargo target dependency requires these files.

- [x] **Step 2: Delete the five approved files**

Delete the exact files listed above with patch-based deletion. Do not delete
`examples/validate_shader.rs`, `src/capture_windows.rs`, `src/gpu_share.rs`,
`src/screenshot_fix.rs`, or anything under `src/platform/windows/`.

- [x] **Step 3: Remove empty local directories**

Resolve and verify that these paths are empty and under
`D:\blackholetrash\singularity`, then remove them non-recursively:

```powershell
Remove-Item -LiteralPath D:\blackholetrash\singularity\config
Remove-Item -LiteralPath D:\blackholetrash\singularity\watchdog
```

Expected: both empty directories are gone; no tracked files are removed.

### Task 2: Rename active product files and identifiers

**Files:**
- Move: `assets/singularity.ico` → `assets/black-hole-trash.ico`
- Move: `src/singularity.wgsl` → `src/black_hole_trash.wgsl`
- Modify: `Cargo.toml`
- Modify: `build.rs`
- Modify: `src/main.rs`
- Modify: `src/black_hole_trash.wgsl`
- Modify: `src/capture_macos.rs`
- Modify: `README.md`

- [x] **Step 1: Rename the icon and shader**

Use patch-based moves so Git records their history. Update `build.rs` to load
`assets/black-hole-trash.ico`, and update the renderer and WGSL validator to
include `src/black_hole_trash.wgsl`.

- [x] **Step 2: Update Cargo and Windows metadata**

Set:

```toml
repository = "https://github.com/rrrjqy66/BlackHoleTrash"
description = "A compact Windows black-hole Recycle Bin with real-time gravitational lensing"
```

Keep the package name `black-hole-trash` and binary name `BlackHoleTrash`.
Change active window titles, configuration filename, user agent, release API,
single-instance mutex/event identifiers, shader header, and macOS output class
to Black Hole Trash equivalents.

- [x] **Step 3: Rewrite active README branding**

Change the title and product prose to Black Hole Trash, document direct
dragging and Recycle Bin behavior, remove the deleted demo/ShaderGlass
instructions, and retain this attribution:

```text
Black Hole Trash is based on GreenScreen410/singularity and retains its MIT license.
```

- [x] **Step 4: Scan stale names**

Run:

```powershell
rg -n -i "singularity|GreenScreen410/singularity" -g "!target/**" -g "!Cargo.lock"
```

Expected: matches remain only in the MIT/upstream attribution and the two
historical design/plan documents where the fork origin is intentionally
recorded.

### Task 3: Validate the cleaned application

**Files:**
- Validate: `Cargo.toml`
- Validate: `src/main.rs`
- Validate: `src/black_hole_trash.wgsl`
- Validate: `src/platform/windows/*.rs`

- [x] **Step 1: Format and compile**

Run:

```powershell
cargo fmt --all
cargo check --all-targets
```

Expected: compilation succeeds. The pre-existing Copy-HANDLE warning may
remain; no new warnings from the cleanup are accepted.

- [x] **Step 2: Validate WGSL**

Run:

```powershell
cargo run --example validate_shader
```

Expected: the renamed WGSL shader parses and validates successfully.

- [x] **Step 3: Build the Windows release executable**

Run:

```powershell
cargo build --release --bin BlackHoleTrash
```

Expected: `target\release\BlackHoleTrash.exe` exists.

- [x] **Step 4: Recheck deletion safety**

Run:

```powershell
rg -n "DeleteFile(W|A)?\s*\(|RemoveDirectory(W|A)?\s*\(|remove_file\s*\(|remove_dir(_all)?\s*\(" src
```

Expected: no permanent-deletion API is present; Recycle Bin behavior remains
implemented only through `IFileOperation` and `FOFX_RECYCLEONDELETE`.

### Task 4: Commit and publish the private repository

**Files:**
- Commit: all approved tracked changes and `src/platform/windows/`
- Exclude: `target/`, temporary files, credentials

- [ ] **Step 1: Review the exact publication diff**

Run:

```powershell
git status -sb
git diff --check
git diff --stat
```

Expected: only the approved product work, cleanup, rename, documentation, and
planning files are present.

- [ ] **Step 2: Commit the cleaned product**

Stage explicit approved paths and commit:

```powershell
git commit -m "Build Black Hole Trash recycle bin"
```

Expected: the commit succeeds on `main`.

- [ ] **Step 3: Create the private GitHub repository**

Run:

```powershell
gh repo create rrrjqy66/BlackHoleTrash --private --description "A compact Windows black-hole Recycle Bin with real-time gravitational lensing"
```

Expected: GitHub creates `rrrjqy66/BlackHoleTrash` with private visibility.

- [ ] **Step 4: Preserve upstream and push**

Run:

```powershell
git remote rename origin upstream
git remote add origin https://github.com/rrrjqy66/BlackHoleTrash.git
git push -u origin main
```

Expected: `main` tracks `origin/main`; `upstream` still points to
`GreenScreen410/singularity`.

- [ ] **Step 5: Verify publication**

Run:

```powershell
gh repo view rrrjqy66/BlackHoleTrash --json nameWithOwner,visibility,url,defaultBranchRef
git remote -v
git status -sb
```

Expected: the repository is `PRIVATE`, default branch is `main`, both remotes
are correct, and the working tree is clean.
