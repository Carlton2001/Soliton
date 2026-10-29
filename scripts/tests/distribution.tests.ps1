# Exercise real ZIP assembly with a synthetic payload; no Rust build or installation.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$fixture = Join-Path $projectRoot "target/distribution-tests/$([Guid]::NewGuid().ToString('N'))"
foreach ($relative in @('scripts', 'installer/windows', 'licenses/dependencies/assets', 'target/x86_64-pc-windows-msvc/release')) {
    New-Item -ItemType Directory -Path (Join-Path $fixture $relative) -Force | Out-Null
}
foreach ($script in @('build-distribution.ps1', 'write-update-manifest.ps1')) {
    Copy-Item -LiteralPath (Join-Path $projectRoot "scripts/$script") -Destination (Join-Path $fixture 'scripts')
}
foreach ($script in @('update-third-party-licenses.ps1', 'check-third-party-licenses.ps1')) {
    [IO.File]::WriteAllText((Join-Path $fixture "scripts/$script"), '# Stub: license generation tested separately.')
}
foreach ($relative in @('LICENSE', 'licenses/dependencies/THIRD-PARTY-NOTICES.md', 'licenses/dependencies/assets/LUCIDE-LICENSE.txt')) {
    [IO.File]::WriteAllText((Join-Path $fixture $relative), 'Fixture license notice')
}
[IO.File]::WriteAllText((Join-Path $fixture 'installer/windows/README.txt'), 'Fixture readme')
$executable = Join-Path $fixture 'target/x86_64-pc-windows-msvc/release/Soliton.exe'
[IO.File]::WriteAllText($executable, 'Synthetic executable fixture')
function cargo {
    if (($args -join ' ') -ne 'metadata --locked --offline --no-deps --format-version 1') { throw 'Unexpected cargo invocation.' }
    $global:LASTEXITCODE = 0
    '{"packages":[{"name":"soliton-app","version":"1.2.3"}]}'
}
$compiler = Join-Path $fixture 'scripts/fake-iscc.ps1'
[IO.File]::WriteAllText($compiler, @'
param([Parameter(ValueFromRemainingArguments)][string[]]$Arguments)
$payload = ($Arguments | Where-Object { $_.StartsWith('/DPayloadDir=') }).Substring(13)
$output = ($Arguments | Where-Object { $_.StartsWith('/O') }).Substring(2)
if (Test-Path -LiteralPath (Join-Path $payload 'data')) { throw 'Portable data would leak into Setup.' }
if (-not (Test-Path -LiteralPath (Join-Path $payload 'LICENSE'))) { throw 'Root license missing in Setup.' }
[IO.File]::WriteAllText((Join-Path $output 'Soliton_1.2.3_x64-setup.exe'), 'Synthetic Setup fixture')
$global:LASTEXITCODE = 0
'@)
& (Join-Path $fixture 'scripts/build-distribution.ps1') -SkipBuild -IsccPath $compiler
$output = Join-Path $fixture 'target/distribution/output'
foreach ($name in @('Soliton-1.2.3-x64-portable.zip', 'Soliton_1.2.3_x64-setup.exe')) {
    $file = Join-Path $output $name
    $actual = (Get-FileHash -LiteralPath $file).Hash.ToLowerInvariant()
    if ((Get-Content -LiteralPath "$file.sha256" -Raw).Trim() -ne "$actual  $name") { throw "Incorrect checksum for $name" }
}
$zip = [IO.Compression.ZipFile]::OpenRead((Join-Path $output 'Soliton-1.2.3-x64-portable.zip'))
try {
    foreach ($name in @('Soliton.exe', 'LICENSE', 'README.txt', 'data/README.txt', 'licenses/dependencies/THIRD-PARTY-NOTICES.md', 'licenses/dependencies/assets/LUCIDE-LICENSE.txt')) {
        if (-not $zip.GetEntry("Soliton/$name")) { throw "Missing ZIP entry: $name" }
    }
} finally { $zip.Dispose() }
& (Join-Path $fixture 'scripts/build-distribution.ps1') -SkipBuild -PortableOnly
[IO.File]::WriteAllText($executable, 'Imports VCRUNTIME140.dll')
$rejected = $false
try { & (Join-Path $fixture 'scripts/build-distribution.ps1') -SkipBuild -PortableOnly } catch { $rejected = $true }
if (-not $rejected) { throw 'Dynamic MSVC runtime accepted.' }
Write-Host 'Distribution assembly tests passed.'
