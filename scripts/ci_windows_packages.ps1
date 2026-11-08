$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not $IsWindows) { throw 'Run Windows package checks with PowerShell 7 on Windows.' }
function Invoke-Checked([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed (exit $LASTEXITCODE)" }
}
$meta = Get-Content .tmp/release-candidate/release.json -Raw | ConvertFrom-Json
if ($meta.development -ne $true) { throw 'Only development CI packages may enter this workflow.' }
if ($env:CARGO_HOME) {
    $env:npm_config_cache = Join-Path (Split-Path $env:CARGO_HOME -Parent) 'windows-package-npm-v1'
} else { $env:npm_config_cache = Join-Path $PWD '.tmp/npm-cache' }
Invoke-Checked 'npm.cmd' @('install','--prefix','.tmp/host-tools','--ignore-scripts','--no-audit','--no-fund','--prefer-offline','--registry=https://registry.npmjs.org','opencode-ai@1.19.4','@openai/codex@0.164.0')
$codex = Join-Path $PWD '.tmp/host-tools/node_modules/@openai/codex/bin/codex.js'
$opencode = Join-Path $PWD '.tmp/host-tools/node_modules/opencode-windows-x64/bin/opencode.exe'
Invoke-Checked 'node' @('scripts/check_windows_packages.mjs','--release','.tmp/release-candidate','--codex-entry',$codex,'--opencode',$opencode)
