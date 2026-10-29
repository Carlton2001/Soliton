# Set the package version, commit it and create the matching release tag.
# Pushing the tag (done by hand) starts the release workflow.
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))

function Invoke-Git {
    $output = & git @args
    if ($LASTEXITCODE -ne 0) { throw "git $($args -join ' ') failed." }
    $output
}

if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw 'Use a numeric major.minor.patch version, e.g. 2.0.0.' }
$tag = "v$Version"

Push-Location -LiteralPath $projectRoot
try {
    if ((Invoke-Git branch --show-current) -ne 'main') { throw 'Release from the main branch.' }
    if (Invoke-Git status --porcelain) { throw 'Commit or stash local changes first.' }
    if (Invoke-Git tag --list $tag) { throw "Tag $tag already exists locally." }
    if (Invoke-Git ls-remote --tags origin "refs/tags/$tag") { throw "Tag $tag already exists on origin." }

    $manifest = Join-Path $projectRoot 'Cargo.toml'
    $text = [IO.File]::ReadAllText($manifest)
    $newline = if ($text.Contains("`r`n")) { "`r`n" } else { "`n" }
    $lines = $text.TrimEnd("`r", "`n") -split "`r?`n"
    $index = [Array]::FindIndex($lines, [Predicate[string]] { param($line) $line -match '^version\s*=\s*"' })
    if ($index -lt 0) { throw 'Package version not found in Cargo.toml.' }
    $current = $lines[$index] -replace '^version\s*=\s*"([^"]+)".*$', '$1'
    if ([version]$Version -le [version]$current) { throw "Version $Version must be greater than $current." }
    $lines[$index] = "version = `"$Version`""
    [IO.File]::WriteAllText($manifest, ($lines -join $newline) + $newline)

    & cargo update --workspace --offline
    if ($LASTEXITCODE -ne 0) { throw 'Cargo.lock update failed.' }
    # The notices record the Cargo.lock hash, which the version change alters.
    & (Join-Path $PSScriptRoot 'update-third-party-licenses.ps1') -Offline

    # The Linux packages require an AppStream release for their version.
    $metainfo = Join-Path $projectRoot 'installer/linux/io.github.Carlton2001.Soliton.metainfo.xml'
    $xml = [IO.File]::ReadAllText($metainfo)
    $date = [DateTime]::UtcNow.ToString('yyyy-MM-dd')
    $updated = [regex]::Replace($xml, '(?m)^(\s*)<releases>\r?$', { param($m) "$($m.Value)`n$($m.Groups[1].Value)  <release version=`"$Version`" date=`"$date`"/>" }, 'None')
    if ($updated -eq $xml) { throw 'AppStream releases not found.' }
    # Screenshot URLs follow the tag, which Flathub reads when it builds.
    $updated = $updated -replace '/Soliton/v[0-9.]*/screenshots/', "/Soliton/$tag/screenshots/"
    [IO.File]::WriteAllText($metainfo, $updated, [Text.UTF8Encoding]::new($false))

    Invoke-Git add Cargo.toml Cargo.lock installer/linux/io.github.Carlton2001.Soliton.metainfo.xml licenses/dependencies | Out-Null
    Invoke-Git commit -m "Release $tag" | Out-Null
    Invoke-Git tag -a $tag -m "Soliton $tag" | Out-Null
    Write-Host "Version $current -> $Version committed and tagged $tag." -ForegroundColor Green
    Write-Host "Publish with: git push origin main $tag" -ForegroundColor Yellow
} finally { Pop-Location }

