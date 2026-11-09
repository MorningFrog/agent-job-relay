[CmdletBinding()]
param([Parameter(Mandatory=$true)][ValidateSet('toolchain','frontend','native','packages')][string]$Step)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
& node scripts/release_artifacts.mjs guard
if ($LASTEXITCODE -ne 0) { throw 'Manual exact-source release context is required.' }
# A different namespace from automatic CI. The account still runs trusted code.
$base = if ($env:CARGO_HOME) { Split-Path $env:CARGO_HOME -Parent } else { Join-Path $PWD '.tmp' }
$env:AJR_CI_CACHE_ROOT = Join-Path $base 'release-msvc-rust1.99.1-node24-v1'
if ($Step -eq 'packages') {
    $env:CARGO_HOME = Join-Path $env:AJR_CI_CACHE_ROOT 'cargo'
    & ./scripts/ci_windows_packages.ps1 -ReleasePreparation
} else {
    & ./scripts/ci_windows.ps1 $Step
}
