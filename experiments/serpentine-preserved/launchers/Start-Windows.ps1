# An isolated source-build launcher. No administrator rights or global PATH edits.
[CmdletBinding()]
param(
    [string]$SourceRoot = (Join-Path (Split-Path $PSScriptRoot -Parent) 'Serpentine3D'),
    [switch]$SetupOnly,
    [switch]$SelfTest,
    [switch]$PythonBackend
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if (-not [Environment]::Is64BitOperatingSystem -or $env:PROCESSOR_ARCHITECTURE -eq 'ARM64') {
    throw 'This launcher supports x64 Windows. ARM64 is not certified.'
}
$SourceRoot = (Resolve-Path -LiteralPath $SourceRoot).Path
if (-not (Test-Path -LiteralPath (Join-Path $SourceRoot 'uv.lock'))) {
    throw 'The complete Serpentine3D source folder and uv.lock are required.'
}
$runtime = Join-Path (Split-Path $SourceRoot -Parent) '.runtime'
New-Item -ItemType Directory -Force -Path $runtime | Out-Null
$gate = $null
$archive = $null
try {
    try {
        $gate = [IO.File]::Open((Join-Path $runtime 'setup.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
    } catch {
        throw 'Another launcher is preparing this kit, or its directory is not writable.'
    }
    $version = '0.13.7'
    $expected = '0cdc5b38fe5a8168ed6a25814f5ba43c21d313dd3187314fccb37b7cec46e021'
    $zip = Join-Path $runtime "uv-$version-windows-x64.zip"
    if (-not (Test-Path -LiteralPath $zip)) {
        $partial = Join-Path $runtime ([Guid]::NewGuid().ToString('N') + '.download')
        try {
            Write-Host 'Downloading the pinned uv bootstrap. Initial setup also downloads Python and CAD dependencies.'
            [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
            Invoke-WebRequest -UseBasicParsing -TimeoutSec 180 -Uri "https://github.com/astral-sh/uv/releases/download/$version/uv-x86_64-pc-windows-msvc.zip" -OutFile $partial
            if ((Get-FileHash -LiteralPath $partial -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) {
                throw 'Bootstrap archive checksum mismatch; nothing was executed.'
            }
            Move-Item -LiteralPath $partial -Destination $zip
        } finally {
            if (Test-Path -LiteralPath $partial) { Remove-Item -LiteralPath $partial }
        }
    }
    if ((Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expected) {
        throw 'Cached bootstrap checksum mismatch. Remove only the cached uv ZIP and retry.'
    }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead($zip)
    $entries = @($archive.Entries | Where-Object { $_.FullName -eq 'uv.exe' })
    if ($entries.Count -ne 1) { throw 'The pinned archive has an unexpected layout.' }
    $uv = Join-Path $runtime 'uv.exe'
    $stream = $entries[0].Open()
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $exeHash = [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '').ToLowerInvariant() }
    finally { $stream.Dispose(); $sha.Dispose() }
    $needsCopy = -not (Test-Path -LiteralPath $uv)
    if (-not $needsCopy) { $needsCopy = (Get-FileHash -LiteralPath $uv -Algorithm SHA256).Hash.ToLowerInvariant() -ne $exeHash }
    if ($needsCopy) { [IO.Compression.ZipFileExtensions]::ExtractToFile($entries[0], $uv, $true) }
    $archive.Dispose(); $archive = $null
    # All installation/cache paths are local to this kit. Python is managed here.
    $env:UV_CACHE_DIR = Join-Path $runtime 'cache'
    $env:UV_PYTHON_INSTALL_DIR = Join-Path $runtime 'python'
    $env:UV_PROJECT_ENVIRONMENT = Join-Path $runtime 'venv'
    $env:PYTHONUTF8 = '1'
    Push-Location -LiteralPath $SourceRoot
    try {
        & $uv sync --locked --no-dev --managed-python --python 3.12
        if ($LASTEXITCODE -ne 0) { throw "Runtime preparation failed (exit $LASTEXITCODE)." }
    } finally { Pop-Location }
} finally {
    if ($null -ne $archive) { $archive.Dispose() }
    if ($null -ne $gate) { $gate.Dispose() }
}
if ($SetupOnly) { Write-Host 'Isolated runtime prepared.'; exit 0 }
if ($PythonBackend) { $env:SERP3D_PICK_BACKEND = 'python' }
Push-Location -LiteralPath $SourceRoot
try {
    if ($SelfTest) {
        $env:QT_QPA_PLATFORM = 'offscreen'
        $env:SERP3D_NO_RPC = '1'
        & $uv run --frozen --no-dev --managed-python python -m serpentine3d --selftest
        if ($LASTEXITCODE -ne 0) { throw "Upstream runtime self-test failed (exit $LASTEXITCODE)." }
        & $uv run --frozen --no-dev --managed-python python -c "from serpentine3d.core.native_spatial import backend_status; s=backend_status(); print(s); assert s['available'], s"
        if ($LASTEXITCODE -ne 0) { throw 'The packaged native accelerator did not load.' }
    } else {
        & $uv run --frozen --no-dev --managed-python python -m serpentine3d
        if ($LASTEXITCODE -ne 0) { throw "Serpentine3D stopped with exit code $LASTEXITCODE." }
    }
} finally { Pop-Location }
