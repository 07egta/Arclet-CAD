<#
  Package only Arclet CAD's already-built Windows x64 executable.
  Does not touch Melt3D, SolidCraft or YURACAD binaries or releases.
#>
param(
    [string] $OutputDir = 'dist/arclet-windows',
    [string] $SourceSha = $env:GITHUB_SHA
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Push-Location $root
try {
    $metadata = & cargo metadata --no-deps --format-version 1 --locked
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $pkg = ($metadata | ConvertFrom-Json).packages |
        Where-Object { $_.name -eq 'arclet' } | Select-Object -First 1
    if (-not $pkg) { throw 'arclet package is missing from the workspace' }
    $version = [string]$pkg.version
    if ($version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+([+-][A-Za-z0-9.-]+)?$') {
        throw "Unexpected Arclet version: $version"
    }
    $exe = Join-Path $root 'target/release/arclet.exe'
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "Missing Arclet executable: $exe" }
    $bytes = [System.IO.File]::ReadAllBytes($exe)
    if ($bytes.Length -lt 1MB -or $bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A) {
        throw 'Arclet is not a plausible Windows PE executable'
    }
    $offset = [BitConverter]::ToInt32($bytes, 0x3C)
    if ($offset -lt 0 -or $offset + 0x60 -ge $bytes.Length) { throw 'Invalid PE header offset' }
    if ([BitConverter]::ToUInt32($bytes, $offset) -ne 0x00004550) { throw 'PE signature mismatch' }
    if ([BitConverter]::ToUInt16($bytes, $offset + 4) -ne 0x8664) { throw 'Expected Windows x86_64 executable' }
    if ([BitConverter]::ToUInt16($bytes, $offset + 0x5C) -ne 2) { throw 'Expected GUI subsystem (2)' }

    $output = [System.IO.Path]::GetFullPath((Join-Path $root $OutputDir))
    New-Item -ItemType Directory -Force -Path $output | Out-Null
    $name = "ArcletCAD-$version-windows-x64"
    $stage = Join-Path $output $name
    if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $stage | Out-Null
    Copy-Item -LiteralPath $exe -Destination (Join-Path $stage 'arclet.exe')
    foreach ($license in @('LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'ATTRIBUTION.md', 'THIRD-PARTY-LICENSES.txt', 'LICENSE_AUDIT.md')) {
        $from = Join-Path $root $license
        if (-not (Test-Path -LiteralPath $from -PathType Leaf)) { throw "Missing required notice: $license" }
        Copy-Item -LiteralPath $from -Destination $stage
    }
    Copy-Item -LiteralPath (Join-Path $root 'apps/arclet/README.md') -Destination (Join-Path $stage 'README-Arclet.md')
    $source = if ($SourceSha) { $SourceSha } else { 'local-unverified' }
    $runUrl = if ($env:GITHUB_RUN_ID) { "$env:GITHUB_SERVER_URL/$env:GITHUB_REPOSITORY/actions/runs/$env:GITHUB_RUN_ID" } else { 'local build; no CI provenance' }
    $compiler = & rustc --version
    if ($LASTEXITCODE -ne 0) { throw 'rustc --version failed' }
    @"
Arclet CAD $version - Windows x64 portable, unsigned development build

Start: extract the ZIP and double-click arclet.exe.
Requires 64-bit Windows 10/11 and a working Vulkan, DirectX 12 or OpenGL driver.
No administrator rights and no installer are required.
NOT a Melt3D or YURACAD installer.

If GPU initialization fails, consult:
  %LOCALAPPDATA%\Arclet\logs\arclet-startup.log
  Update GPU drivers; WGPU_BACKEND=gl can be tried as a diagnostic override.
Back up important CAD files; this is a development build.

Arclet version: $version
Source commit: $source
Build workflow: $runUrl
Rust compiler: $compiler
Licences: MIT OR Apache-2.0; third-party notices included.
Unsigned. CI build and GUI screenshot tests do not prove full user acceptance.
"@ | Set-Content -LiteralPath (Join-Path $stage 'INSTALL.txt') -Encoding UTF8

    $hashLines = Get-ChildItem -LiteralPath $stage -File |
        Sort-Object Name |
        ForEach-Object { "$((Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant())  $($_.Name)" }
    $hashLines | Set-Content -LiteralPath (Join-Path $stage 'SHA256SUMS.txt') -Encoding ascii
    $zip = Join-Path $output "$name.zip"
    if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip -Force }
    Compress-Archive -LiteralPath $stage -DestinationPath $zip -CompressionLevel Optimal
    $zipHash = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    "$zipHash  $name.zip" | Set-Content -LiteralPath "$zip.sha256" -Encoding ascii
    if ((Get-Item -LiteralPath $zip).Length -lt 1MB) { throw 'Unexpectedly small Arclet ZIP' }
    Write-Output "Arclet ZIP: $zip"
    Write-Output "SHA-256: $zipHash"
} finally {
    Pop-Location
}
