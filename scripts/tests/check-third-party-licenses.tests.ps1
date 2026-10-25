$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
$fixture = Join-Path $projectRoot "target/license-check-tests/$([Guid]::NewGuid().ToString('N'))"
foreach ($directory in @('scripts', 'licenses/dependencies', 'crates/example', 'vendor/example')) {
    New-Item -ItemType Directory -Path (Join-Path $fixture $directory) -Force | Out-Null
}
$checker = Join-Path $fixture 'scripts/check-third-party-licenses.ps1'
Copy-Item -LiteralPath (Join-Path $projectRoot 'scripts/check-third-party-licenses.ps1') -Destination $checker
$report = Join-Path $fixture 'licenses/dependencies/THIRD-PARTY-NOTICES.md'
[IO.File]::WriteAllText($report, 'Original license text')
$hashes = [ordered]@{ 'licenses/dependencies/THIRD-PARTY-NOTICES.md' = (Get-FileHash $report).Hash }
$inventory = Join-Path $fixture 'licenses/dependencies/inputs.json'
[IO.File]::WriteAllText($inventory, ($hashes | ConvertTo-Json))
& $checker

function Assert-Rejected([string]$Scenario) {
    $rejected = $false
    try { & $checker } catch { $rejected = $true }
    if (-not $rejected) { throw "Checker accepted $Scenario." }
}
[IO.File]::WriteAllText($report, 'Changed license text')
Assert-Rejected 'a modified report'
[IO.File]::WriteAllText($report, 'Original license text')
foreach ($relative in @('crates/example/Cargo.toml', 'vendor/example/Cargo.toml')) {
    $path = Join-Path $fixture $relative
    [IO.File]::WriteAllText($path, '[package]')
    Assert-Rejected "an added manifest: $relative"
    $hashes[$relative] = (Get-FileHash $path).Hash
    [IO.File]::WriteAllText($inventory, ($hashes | ConvertTo-Json))
    & $checker
}
Remove-Item -LiteralPath $report
Assert-Rejected 'a missing report'
Write-Host 'License checker tests passed.'
