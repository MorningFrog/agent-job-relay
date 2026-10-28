param([Parameter(Mandatory=$true)][string]$Binary,[Parameter(Mandatory=$true)][string]$Output)
$ErrorActionPreference='Stop'
$version = (& $Binary --version).Trim()
if ($LASTEXITCODE -ne 0 -or $version -notmatch '^agent-job-relay (\d+\.\d+\.\d+)$') { throw 'Native executable version check failed' }
$nativeVersion=$Matches[1]
$description=@{format=1;platform='win32-x64';version=$nativeVersion;sha256=(Get-FileHash -LiteralPath $Binary -Algorithm SHA256).Hash.ToLowerInvariant();revision=$env:GITHUB_SHA}
$destination=if([IO.Path]::IsPathRooted($Output)){$Output}else{Join-Path (Get-Location).ProviderPath $Output}
[IO.File]::WriteAllText([IO.Path]::GetFullPath($destination),($description | ConvertTo-Json),[Text.UTF8Encoding]::new($false))
