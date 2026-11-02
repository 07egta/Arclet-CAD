<#
  Starts the real Arclet Windows GUI/WGPU renderer and captures UI frames.
  Does not assert that a human edited, saved or reopened a design.
#>
param(
    [string] $Executable = 'target/release/arclet.exe',
    [string] $OutputDir = ''
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$exe = (Resolve-Path -LiteralPath $Executable).Path
if (-not $OutputDir) {
    $tempRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [System.IO.Path]::GetTempPath() }
    $OutputDir = Join-Path $tempRoot 'arclet-smoke'
}
New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
foreach ($case in @(
    @{ Name = 'model'; Arguments = '--sample' },
    @{ Name = 'nodes'; Arguments = '--nodes' },
    @{ Name = 'curve'; Arguments = '--curve-sample' }
)) {
    $shot = Join-Path $OutputDir "$($case.Name).png"
    $arguments = $case.Arguments + ' --screenshot "' + $shot + '"'
    Write-Output "Launching Arclet GUI: $arguments"
    $process = Start-Process -FilePath $exe -ArgumentList $arguments -PassThru
    try {
        if (-not $process.WaitForExit(120000)) { throw "Arclet GUI $($case.Name) timed out" }
        if ($process.ExitCode -ne 0) { throw "Arclet GUI $($case.Name) exited with $($process.ExitCode)" }
        if (-not (Test-Path -LiteralPath $shot -PathType Leaf)) { throw "Arclet GUI $($case.Name) did not render a PNG" }
        $bytes = [System.IO.File]::ReadAllBytes($shot)
        if ($bytes.Length -lt 10000 -or
            $bytes[0] -ne 137 -or $bytes[1] -ne 80 -or $bytes[2] -ne 78 -or $bytes[3] -ne 71) {
            throw "Arclet GUI $($case.Name) screenshot is invalid or empty"
        }
        $w = [uint32](([uint32]$bytes[16] -shl 24) -bor ([uint32]$bytes[17] -shl 16) -bor ([uint32]$bytes[18] -shl 8) -bor [uint32]$bytes[19])
        $h = [uint32](([uint32]$bytes[20] -shl 24) -bor ([uint32]$bytes[21] -shl 16) -bor ([uint32]$bytes[22] -shl 8) -bor [uint32]$bytes[23])
        if ($w -lt 800 -or $h -lt 500) { throw "Arclet GUI $($case.Name) screenshot is too small: $($w)x$h" }
        Write-Output "Rendered $($case.Name): $($w)x$h, $($bytes.Length) bytes"
    } finally {
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        }
        $process.Dispose()
    }
}
