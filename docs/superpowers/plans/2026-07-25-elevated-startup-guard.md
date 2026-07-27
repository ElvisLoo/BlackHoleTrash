# Elevated Startup Guard Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refuse to run Black Hole Trash with an elevated Windows token and explain how to launch normally.

**Architecture:** Add one Windows-only process-token module and call it before single-instance takeover. Keep the installer unchanged and document the restriction in both READMEs.

**Tech Stack:** Rust, windows-rs Win32 Security API, MessageBoxW.

---

### Task 1: Detect and block elevation

**Files:** Create `src/platform/windows/process_elevation.rs`; modify `src/platform/windows/mod.rs` and `src/main.rs`.

- [ ] Add failing tests for `elevation_from_flag(0)`, nonzero flags, and both warning messages.
- [ ] Run `cargo test process_elevation -- --nocapture`; expect RED because the module/API is absent.
- [ ] Implement `OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY)` plus `GetTokenInformation(TokenElevation)` and close the token handle on every query path.
- [ ] Implement `allow_startup()`: standard token returns `true`; elevated or query failure shows a foreground warning and returns `false`.
- [ ] Call `allow_startup()` immediately after `env_logger::init()` and return before single-instance takeover when it is false.
- [ ] Run focused tests, then commit as `fix: block elevated startup`.

### Task 2: Troubleshooting and verification

**Files:** Modify `README.md` and `README.en.md`.

- [ ] Add troubleshooting text stating that administrator launch is unsupported because UIPI blocks normal Explorer drag/drop; instruct direct double-click or Start menu launch.
- [ ] Run `cargo fmt --all -- --check`, `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo check --all-targets`, and `cargo build --release`.
- [ ] Confirm `installer/BlackHoleTrash.iss` still contains `PrivilegesRequired=lowest` and commit as `docs: explain non-elevated startup requirement`.
