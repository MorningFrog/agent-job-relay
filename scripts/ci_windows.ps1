[CmdletBinding()]
param([Parameter(Mandatory=$true)][ValidateSet('toolchain','frontend','frontend-tests','native')][string]$Step)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
Set-Location (Split-Path $PSScriptRoot -Parent)
if (-not $IsWindows) { throw 'Run Windows native CI with PowerShell 7 on Windows.' }

function Invoke-Checked([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed (exit $LASTEXITCODE)" }
}

# The dedicated, capacity-one Worker supplies its project-only Cargo home.
# Retain downloads and compiler artifacts, never fixture state. Check reuse uses independent Server evidence.
$cacheRoot = if ($env:AJR_CI_CACHE_ROOT) { $env:AJR_CI_CACHE_ROOT } elseif ($env:CARGO_HOME) {
    Join-Path (Split-Path $env:CARGO_HOME -Parent) 'native-msvc-rust1.99.1-node24-v1'
} else { Join-Path $PWD '.tmp/ci-windows' }
$env:AJR_CI_CACHE_ROOT = $cacheRoot
$env:CARGO_HOME = Join-Path $cacheRoot 'cargo'
$env:npm_config_cache = Join-Path $cacheRoot 'npm'
$env:CARGO_TARGET_DIR = Join-Path $cacheRoot 'target'
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_INCREMENTAL = '1'
$env:CARGO_BUILD_JOBS = if ($env:AJR_CI_BUILD_JOBS) { $env:AJR_CI_BUILD_JOBS } else { [string][Environment]::ProcessorCount }
New-Item -ItemType Directory -Path $env:CARGO_HOME,$env:npm_config_cache,$env:CARGO_TARGET_DIR -Force | Out-Null

# Import only this process's compiler environment; leave global PATH untouched.
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$installation = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $installation) { throw 'Visual Studio C++ x64 tools are required.' }
$developer = Join-Path $installation 'Common7/Tools/VsDevCmd.bat'
# A batch file avoids PowerShell 7's native-argument quoting around paths with
# spaces. cmd receives one simple relative filename rather than nested quotes.
New-Item -ItemType Directory -Path .tmp -Force | Out-Null
[IO.File]::WriteAllText((Join-Path $PWD '.tmp/ci-msvc-env.cmd'), "@call `"$developer`" -no_logo -arch=amd64 -host_arch=amd64 >nul && set`r`n")
$compilerEnvironment = & $env:ComSpec /d /c .tmp\ci-msvc-env.cmd
if ($LASTEXITCODE -ne 0) { throw 'Cannot initialize the MSVC environment.' }
foreach ($line in $compilerEnvironment) {
    if ($line -match '^([^=]+)=(.*)$') { [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], 'Process') }
}

switch ($Step) {
    'toolchain' {
        foreach ($tool in @('node','npm.cmd','git','rustc','cargo','cl.exe','link.exe')) { (Get-Command $tool -ErrorAction Stop).Source }
        if ((& rustc --version) -notmatch '^rustc 1\.99\.1 ') { throw 'Expected Rust 1.99.1.' }
        if ((& node --version) -notmatch '^v24\.') { throw 'Expected Node 24.' }
        Invoke-Checked 'cargo' @('--version')
        Invoke-Checked 'cargo' @('clippy','--version')
        Invoke-Checked 'cargo' @('fmt','--version')
        Invoke-Checked 'git' @('--version')
        $revision = & git rev-parse HEAD
        if ($LASTEXITCODE -ne 0) { throw 'The checkout must retain Git source identity.' }
        if ($env:CI_COMMIT_SHA -and $revision -ne $env:CI_COMMIT_SHA) { throw 'Checkout differs from the requested CI commit.' }
        Write-Output "Windows native CI: $revision; $env:CARGO_BUILD_JOBS build jobs; cache $cacheRoot"
    }
    'frontend' {
        Invoke-Checked 'npm.cmd' @('ci','--prefix','web','--ignore-scripts','--no-audit','--no-fund','--prefer-offline')
        Invoke-Checked 'node' @('.ci/ci_checks.mjs','frontend-windows')
        Invoke-Checked 'npm.cmd' @('run','build','--prefix','web')
    }
    'frontend-tests' {
        Invoke-Checked 'npm.cmd' @('test','--prefix','web')
        Invoke-Checked 'node' @('--test','scripts/test_online_update.mjs','scripts/test_online_uninstall.mjs','scripts/test_ci_artifacts.mjs','scripts/test_release_artifacts.mjs','scripts/test_ci_inputs.mjs','scripts/test_opencode_ready.mjs')
    }
    'native' {
        $env:WP_INPUT_MTIMES_FILE = Join-Path $cacheRoot 'input-mtimes.json'
        Invoke-Checked 'node' @('.ci/ci_inputs.mjs','restore')
        Invoke-Checked 'cargo' @('fmt','--check')
        Invoke-Checked 'cargo' @('clippy','--all-targets','--locked','--','-D','warnings')
        Invoke-Checked 'node' @('.ci/ci_checks.mjs','native-windows')
        Invoke-Checked 'cargo' @('build','--release','--locked')
        Invoke-Checked (Join-Path $env:CARGO_TARGET_DIR 'release/agent-job-relay.exe') @('--help')
        Invoke-Checked 'node' @('.ci/ci_inputs.mjs','record')
        New-Item -ItemType Directory -Path target -Force | Out-Null
        $binary = Join-Path $env:CARGO_TARGET_DIR 'release/agent-job-relay.exe'
        $revision = & git rev-parse HEAD
        if ($LASTEXITCODE -ne 0) { throw 'Cannot describe the native source revision.' }
        & ./scripts/describe_native.ps1 -Binary $binary -Output target/windows-native.json -Revision $revision
        # Keep this run's exact bytes in its disposable workspace for the next
        # migration stage; shared compiler output is not an artifact identity.
        New-Item -ItemType Directory -Path target/release -Force | Out-Null
        Copy-Item -LiteralPath $binary -Destination target/release/agent-job-relay.exe
    }
}
