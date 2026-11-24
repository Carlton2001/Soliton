$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$fixture = Join-Path $projectRoot "target/update-manifest-tests/$([Guid]::NewGuid().ToString('N'))"
New-Item -ItemType Directory -Path $fixture -Force | Out-Null
$path = Join-Path $fixture 'latest.json'
$setup = Join-Path $fixture 'Soliton_1.2.3_x64-setup.exe'
$archive = Join-Path $fixture 'Soliton-1.2.3-x86_64-linux.tar.gz'
[IO.File]::WriteAllText($setup, 'Synthetic setup')
[IO.File]::WriteAllText("$setup.sig", 'signed-fixture')
[IO.File]::WriteAllText($archive, 'Synthetic archive')
[IO.File]::WriteAllText("$archive.sig", 'signed-archive')
$arguments = @{ Destination = $path; Setup = $setup; LinuxArchive = $archive; Repository = 'owner/Soliton' }
$writer = Join-Path $projectRoot 'scripts/write-update-manifest.ps1'
& $writer -Version '1.2.3' @arguments
$manifest = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
if ($manifest.version -ne '1.2.3' -or -not $manifest.pub_date) { throw 'Invalid update manifest.' }
if ($manifest.platforms.'windows-x86_64'.signature -ne 'signed-fixture' -or $manifest.platforms.'windows-x86_64'.url -ne 'https://github.com/owner/Soliton/releases/download/v1.2.3/Soliton_1.2.3_x64-setup.exe') { throw 'Invalid signed asset.' }
if ($manifest.platforms.'linux-x86_64'.signature -ne 'signed-archive' -or $manifest.platforms.'linux-x86_64'.url -ne 'https://github.com/owner/Soliton/releases/download/v1.2.3/Soliton-1.2.3-x86_64-linux.tar.gz') { throw 'Invalid signed archive.' }
$rejected = $false
try { & $writer -Version '../bad' @arguments } catch { $rejected = $true }
if (-not $rejected) { throw 'Unsafe version accepted.' }
foreach ($asset in $setup, $archive) {
    $signature = [IO.File]::ReadAllText("$asset.sig")
    [IO.File]::WriteAllText("$asset.sig", '')
    $rejected = $false
    try { & $writer -Version '1.2.3' @arguments } catch { $rejected = $true }
    if (-not $rejected) { throw "Unsigned release asset accepted ($asset)." }
    [IO.File]::WriteAllText("$asset.sig", $signature)
}
Write-Host 'Update manifest tests passed.'
