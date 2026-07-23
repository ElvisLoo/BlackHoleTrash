# Black Hole Trash Windows Installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `BlackHoleTrash-Setup-x64.exe`, generate its SHA-256 checksum, and publish both files in a public GitHub Release `v1.0.0`.

**Architecture:** Inno Setup packages the already validated Rust release executable into a per-user Windows installer. A PowerShell build script provides a reproducible one-command pipeline, while GitHub Release hosts the installer and checksum for unauthenticated public downloads.

**Tech Stack:** Rust 2021, Cargo, Inno Setup 6, PowerShell, Git, GitHub CLI.

---

### Task 1: Install and verify the installer compiler

**Files:**
- External build dependency: Inno Setup 6

- [x] **Step 1: Install Inno Setup 6**

Run:

```powershell
winget install --id JRSoftware.InnoSetup -e --silent --accept-package-agreements --accept-source-agreements
```

Expected: installation succeeds and `ISCC.exe` exists under Program Files or
the current user's Local AppData programs directory.

- [x] **Step 2: Verify the compiler**

Run:

```powershell
& 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe' /?
```

Expected: Inno Setup Compiler help is displayed.

### Task 2: Add reproducible installer sources

**Files:**
- Create: `installer/BlackHoleTrash.iss`
- Create: `scripts/build-installer.ps1`

- [x] **Step 1: Create the Inno Setup definition**

Create `installer/BlackHoleTrash.iss` with:

```ini
#define MyAppName "Black Hole Trash"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "rrrjqy66"
#define MyAppURL "https://github.com/rrrjqy66/BlackHoleTrash"
#define MyAppExeName "BlackHoleTrash.exe"

[Setup]
AppId={{E34B31B5-ABB8-4D90-96C6-472685BF40AB}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}/issues
AppUpdatesURL={#MyAppURL}/releases
DefaultDirName={localappdata}\Programs\BlackHoleTrash
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.19041
OutputDir=..\dist
OutputBaseFilename=BlackHoleTrash-Setup-x64
SetupIconFile=..\assets\black-hole-trash.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
CloseApplications=force
RestartApplications=no
AppMutex=Local\BlackHoleTrashOverlay
VersionInfoVersion=1.0.0.0
VersionInfoDescription={#MyAppName} Installer
VersionInfoProductName={#MyAppName}
VersionInfoProductVersion={#MyAppVersion}

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加选项："; Flags: unchecked

[Files]
Source: "..\target\release\BlackHoleTrash.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.en.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Black Hole Trash"; Filename: "{app}\{#MyAppExeName}"
Name: "{autodesktop}\Black Hole Trash"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "启动 Black Hole Trash"; Flags: nowait postinstall skipifsilent
```

- [x] **Step 2: Create the build script**

Create `scripts/build-installer.ps1` with:

```powershell
[CmdletBinding()]
param(
    [string]$IsccPath = 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe'
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$installerPath = Join-Path $repoRoot 'dist\BlackHoleTrash-Setup-x64.exe'
$checksumPath = "$installerPath.sha256"

if (-not (Test-Path -LiteralPath $IsccPath)) {
    throw "Inno Setup compiler not found: $IsccPath"
}

Push-Location $repoRoot
try {
    cargo build --release --bin BlackHoleTrash
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo release build failed with exit code $LASTEXITCODE"
    }

    & $IsccPath (Join-Path $repoRoot 'installer\BlackHoleTrash.iss')
    if ($LASTEXITCODE -ne 0) {
        throw "Inno Setup failed with exit code $LASTEXITCODE"
    }

    $hash = (Get-FileHash -LiteralPath $installerPath -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  BlackHoleTrash-Setup-x64.exe" |
        Set-Content -LiteralPath $checksumPath -Encoding ascii
} finally {
    Pop-Location
}

Get-Item -LiteralPath $installerPath, $checksumPath
```

### Task 3: Document the public download

**Files:**
- Modify: `README.md`
- Modify: `README.en.md`
- Create: `docs/releases/v1.0.0.md`

- [x] **Step 1: Add Chinese and English download links**

Add this release URL to both README files:

```text
https://github.com/rrrjqy66/BlackHoleTrash/releases/download/v1.0.0/BlackHoleTrash-Setup-x64.exe
```

State that the installer is unsigned and may trigger Windows SmartScreen.

- [x] **Step 2: Create Chinese-first release notes**

Create `docs/releases/v1.0.0.md` describing the compact draggable black hole,
OLE file drop, Recycle Bin-only safety, supported Windows versions, installer
contents, checksum asset, and unsigned SmartScreen warning.

### Task 4: Build and validate artifacts

**Files:**
- Generate: `dist/BlackHoleTrash-Setup-x64.exe`
- Generate: `dist/BlackHoleTrash-Setup-x64.exe.sha256`

- [x] **Step 1: Validate Rust and WGSL**

Run:

```powershell
cargo fmt --all
cargo check --all-targets
cargo run --example validate_shader
```

Expected: Rust compilation and WGSL validation succeed.

- [x] **Step 2: Build installer and checksum**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-installer.ps1
```

Expected: both files appear under `dist/`.

- [x] **Step 3: Independently verify SHA-256**

Run:

```powershell
$expected = (Get-Content dist\BlackHoleTrash-Setup-x64.exe.sha256).Split()[0]
$actual = (Get-FileHash dist\BlackHoleTrash-Setup-x64.exe -Algorithm SHA256).Hash.ToLowerInvariant()
if ($expected -ne $actual) { throw 'SHA-256 mismatch' }
```

Expected: no exception and both hashes are identical.

### Task 5: Commit and publish GitHub Release v1.0.0

**Files:**
- Commit: installer source, build script, documentation and plan
- Upload: `dist/BlackHoleTrash-Setup-x64.exe`
- Upload: `dist/BlackHoleTrash-Setup-x64.exe.sha256`

- [ ] **Step 1: Commit and push installer sources**

Run:

```powershell
git add installer scripts README.md README.en.md docs
git commit -m "Add Windows installer"
git push origin main
```

Expected: `main` is clean and synchronized with `origin/main`.

- [ ] **Step 2: Confirm public repository visibility**

Run:

```powershell
gh repo view rrrjqy66/BlackHoleTrash --json visibility,url
```

Expected: visibility is `PUBLIC`.

- [ ] **Step 3: Create the public release**

Run:

```powershell
gh release create v1.0.0 `
  dist\BlackHoleTrash-Setup-x64.exe `
  dist\BlackHoleTrash-Setup-x64.exe.sha256 `
  --repo rrrjqy66/BlackHoleTrash `
  --target main `
  --title "Black Hole Trash v1.0.0" `
  --notes-file docs\releases\v1.0.0.md
```

Expected: a public, non-draft, non-prerelease `v1.0.0` release is created.

- [ ] **Step 4: Verify assets and public download**

Run:

```powershell
gh release view v1.0.0 --repo rrrjqy66/BlackHoleTrash --json url,isDraft,isPrerelease,assets
Invoke-WebRequest `
  -Uri 'https://github.com/rrrjqy66/BlackHoleTrash/releases/download/v1.0.0/BlackHoleTrash-Setup-x64.exe' `
  -Method Head
```

Expected: the release contains both assets and the unauthenticated download
request returns HTTP 200.
