# Adapted from Oxyde-GPUI (Carlton2001, GPL-3.0).
[CmdletBinding()]
param([string]$IsccPath, [switch]$SkipBuild, [switch]$PortableOnly)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$target = 'x86_64-pc-windows-msvc'
$stage = Join-Path $projectRoot 'target/distribution/payload'
$output = Join-Path $projectRoot 'target/distribution/output'
$previousFlags = $env:RUSTFLAGS
Push-Location -LiteralPath $projectRoot
try {
    if (-not $PortableOnly) {
        if (-not $IsccPath) {
            $compiler = Get-Command ISCC.exe -ErrorAction SilentlyContinue
            if ($compiler) { $IsccPath = $compiler.Source }
            else { $IsccPath = "${env:ProgramFiles(x86)}/Inno Setup 6/ISCC.exe" }
        }
        if (-not (Test-Path -LiteralPath $IsccPath)) { throw 'Install Inno Setup 6.3 or newer, or use -IsccPath.' }
    }
    ./scripts/update-third-party-licenses.ps1
    ./scripts/check-third-party-licenses.ps1
    if (-not $SkipBuild) {
        $env:RUSTFLAGS = "$previousFlags -C target-feature=+crt-static".Trim()
        cargo build --locked --release --target $target -p soliton-app --bin Soliton
        if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    }
    $metadata = cargo metadata --locked --offline --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read version.' }
    $version = (($metadata | ConvertFrom-Json).packages | Where-Object name -eq 'soliton-app').version
    if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Expected numeric major.minor.patch.' }
    if (Test-Path -LiteralPath $stage) {
        $resolved = [IO.Path]::GetFullPath($stage)
        if ($resolved -ne [IO.Path]::GetFullPath((Join-Path $projectRoot 'target/distribution/payload'))) { throw 'Unexpected staging directory.' }
        Remove-Item -LiteralPath $resolved -Recurse -Force
    }
    New-Item -ItemType Directory -Path $stage, $output -Force | Out-Null
    Copy-Item -LiteralPath "target/$target/release/Soliton.exe" -Destination $stage
    $imports = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes((Join-Path $stage 'Soliton.exe')))
    if ($imports -match '(?i)(vcruntime|msvcp)140[_a-z0-9]*\.dll') { throw 'Release still imports an MSVC runtime DLL.' }
    Copy-Item -LiteralPath LICENSE -Destination $stage
    New-Item -ItemType Directory -Path (Join-Path $stage 'licenses') -Force | Out-Null
    Copy-Item -LiteralPath licenses/dependencies -Destination (Join-Path $stage 'licenses') -Recurse
    if (-not $PortableOnly) {
        & $IsccPath "/DAppVersion=$version" "/DPayloadDir=$stage" "/O$output" installer/windows/soliton.iss
        if ($LASTEXITCODE -ne 0) { throw 'Installer compilation failed.' }
    }
    # Added after Setup compilation: installed Soliton must use LocalAppData.
    New-Item -ItemType Directory -Path (Join-Path $stage 'data') -Force | Out-Null
    [IO.File]::WriteAllText((Join-Path $stage 'data/README.txt'), "Ce dossier active le mode portable. Conservez-le avec Soliton.exe.`r`nThis directory enables portable mode. Keep it next to Soliton.exe.`r`n")
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $download = Join-Path $output "Soliton-$version-x64-portable.zip"
    if (Test-Path -LiteralPath $download) { Remove-Item -LiteralPath $download }
    $zip = [IO.Compression.ZipFile]::Open($download, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($file in (Get-ChildItem -LiteralPath $stage -Recurse -File)) {
            $relative = $file.FullName.Substring($stage.Length + 1).Replace('\', '/')
            [IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip, $file.FullName, "Soliton/$relative", [IO.Compression.CompressionLevel]::Optimal) | Out-Null
        }
    } finally { $zip.Dispose() }
    $zip = [IO.Compression.ZipFile]::OpenRead($download)
    try {
        foreach ($file in (Get-ChildItem -LiteralPath $stage -Recurse -File)) {
            $relative = $file.FullName.Substring($stage.Length + 1).Replace('\', '/')
            $entry = $zip.GetEntry("Soliton/$relative")
            if (-not $entry -or $entry.Length -ne $file.Length) { throw "Missing or truncated archive entry: $relative" }
        }
    } finally { $zip.Dispose() }
    foreach ($file in (Get-ChildItem -LiteralPath $output -File | Where-Object { $_.Name -eq "Soliton-$version-x64-portable.zip" -or $_.Name -eq "Soliton_${version}_x64-setup.exe" })) {
        $hash = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        [IO.File]::WriteAllText("$($file.FullName).sha256", "$hash  $($file.Name)`n")
    }
} finally {
    $env:RUSTFLAGS = $previousFlags
    Pop-Location
}
