# Black Hole Trash v1.2.0 README and Release Design

## Goal

Publish the current `main` functionality as v1.2.0 and make the GitHub landing page explain the product, its current features, its release history, and its community links without requiring readers to inspect the source tree.

## README Structure

Both `README.md` and `README.en.md` use the approved product-first structure:

1. Project name, concise positioning, badges, and v1.2.0 download action.
2. Current feature set, including safe Windows recycling, real-time lensing, cursor gravity, six growth tiers, Chinese tray controls, and the 30/60 FPS policy.
3. A focused v1.2.0 section describing the new growth, tray localization, and FPS changes.
4. Usage, safety rules, configuration, system requirements, and source-build instructions.
5. Version history for v1.0.0, v1.1.0, and v1.2.0.
6. A locally committed Star History snapshot for `rrrjqy66/BlackHoleTrash`, linked to the live Star History page so the README remains readable when the image service is unavailable.
7. The final section contains the QQ group and donation images side by side. The Chinese README includes QQ group number `1083121107`; the English README keeps the same community links with English labels.

## Assets

The supplied images are copied without image editing so their QR codes remain intact:

- `assets/community/qq-group.jpg`
- `assets/community/donate.jpg`
- `assets/community/star-history.svg` (generated from GitHub stargazer timestamps at release preparation time)

GitHub-compatible HTML controls their displayed width. Alt text and nearby labels preserve basic accessibility when images do not load.

## Release

The v1.2.0 release uses the existing Windows installer workflow. The release artifacts are:

- `BlackHoleTrash-Setup-x64.exe`
- `BlackHoleTrash-Setup-x64.exe.sha256`

The GitHub Release notes summarize the three user-visible additions and retain the existing unsigned-installer warning. The tag and release are created only after the user approves the rendered README preview.

## Validation

- Confirm both README files are valid UTF-8 and reference existing local assets.
- Confirm no stale FPS options or five-tier wording remains.
- Run the Rust test suite and build the Release executable.
- Build the installer and verify its SHA-256 file.
- Render the README locally for user review before committing or pushing README and release changes.
