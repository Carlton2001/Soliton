$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$fixture = Join-Path $projectRoot "target/update-manifest-tests/$([Guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $fixture -Force | Out-Null
$path = Join-Path $fixture 'latest.json'
$setup = Join-Path $fixture 'RAP_1.2.3_x64-setup.exe'
[IO.File]::WriteAllText($setup, 'Synthetic setup')
[IO.File]::WriteAllText("$setup.sig", 'signed-fixture')
& (Join-Path $projectRoot 'scripts/write-update-manifest.ps1') -Version '1.2.3' -Destination $path -Setup $setup -Repository 'owner/RAP'
$manifest = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
if ($manifest.version -ne '1.2.3' -or -not $manifest.pub_date) { throw 'Invalid update manifest.' }
if ($manifest.platforms.'windows-x86_64'.signature -ne 'signed-fixture' -or $manifest.platforms.'windows-x86_64'.url -ne 'https://github.com/owner/RAP/releases/download/v1.2.3/RAP_1.2.3_x64-setup.exe') { throw 'Invalid signed asset.' }
$rejected = $false
try { & (Join-Path $projectRoot 'scripts/write-update-manifest.ps1') -Version '../bad' -Destination $path -Setup $setup -Repository 'owner/RAP' } catch { $rejected = $true }
if (-not $rejected) { throw 'Unsafe version accepted.' }
[IO.File]::WriteAllText("$setup.sig", '')
$rejected = $false
try { & (Join-Path $projectRoot 'scripts/write-update-manifest.ps1') -Version '1.2.3' -Destination $path -Setup $setup -Repository 'owner/RAP' } catch { $rejected = $true }
if (-not $rejected) { throw 'Unsigned installer accepted.' }
Write-Host 'Update manifest tests passed.'
