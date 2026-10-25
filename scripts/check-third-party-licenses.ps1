[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'license-hash.ps1')
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$inventory = Join-Path $projectRoot 'licenses/dependencies/inputs.json'
if (-not (Test-Path -LiteralPath $inventory)) { throw 'Run scripts/update-third-party-licenses.ps1 first.' }
$hashes = Get-Content -LiteralPath $inventory -Raw | ConvertFrom-Json
foreach ($entry in $hashes.PSObject.Properties) {
    $path = Join-Path $projectRoot $entry.Name
    if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or
        (Get-LicenseHash $path) -ne $entry.Value) {
        throw "Third-party notices are stale: $($entry.Name). Run scripts/update-third-party-licenses.ps1."
    }
}
foreach ($manifest in (Get-ChildItem -Path (Join-Path $projectRoot 'crates/*/Cargo.toml'), (Join-Path $projectRoot 'vendor/*/Cargo.toml') -File)) {
    $relative = $manifest.FullName.Substring($projectRoot.Length + 1).Replace('\', '/')
    if ($hashes.PSObject.Properties.Name -notcontains $relative) { throw "Untracked license input: $relative. Regenerate notices." }
}
Write-Host 'Third-party license notices are up to date.'
