[CmdletBinding()]
param(
    [string]$IsccPath = ''
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$installerPath = Join-Path $repoRoot 'dist\BlackHoleTrash-Setup-x64.exe'
$checksumPath = "$installerPath.sha256"

if (-not $IsccPath) {
    $candidates = @(
        'C:\Program Files (x86)\Inno Setup 6\ISCC.exe',
        'C:\Program Files\Inno Setup 6\ISCC.exe',
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe')
    )
    $IsccPath = $candidates |
        Where-Object { Test-Path -LiteralPath $_ } |
        Select-Object -First 1
}

if (-not $IsccPath -or -not (Test-Path -LiteralPath $IsccPath)) {
    throw 'Inno Setup 6 compiler was not found. Install JRSoftware.InnoSetup with winget.'
}

Push-Location $repoRoot
try {
    cargo build --release --bin BlackHoleTrash
    if ($LASTEXITCODE -ne 0) {
        throw "Cargo release build failed with exit code $LASTEXITCODE"
    }

    New-Item -ItemType Directory -Path (Join-Path $repoRoot 'dist') -Force | Out-Null
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
