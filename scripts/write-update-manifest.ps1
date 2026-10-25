# Adapted from Oxyde-GPUI (Carlton2001, GPL-3.0). Signed installer manifest.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version,
    [Parameter(Mandatory)][string]$Destination,
    [Parameter(Mandatory)][ValidatePattern('^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$')][string]$Repository,
    [Parameter(Mandatory)][string]$Setup
)
$ErrorActionPreference = 'Stop'
$expectedName = "RAP_${Version}_x64-setup.exe"
if ([IO.Path]::GetFileName($Setup) -ne $expectedName -or -not (Test-Path -LiteralPath $Setup)) { throw 'Unexpected installer name or missing installer.' }
$signature = (Get-Content -LiteralPath "$Setup.sig" -Raw).Trim()
if (-not $signature) { throw 'Missing installer signature.' }
$manifest = [ordered]@{
    version = $Version
    pub_date = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ss.fffZ')
    platforms = @{ 'windows-x86_64' = @{
        url = "https://github.com/$Repository/releases/download/v$Version/$expectedName"
        signature = $signature
    } }
}
[IO.File]::WriteAllText($ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Destination), ($manifest | ConvertTo-Json -Depth 4) + "`n", [Text.UTF8Encoding]::new($false))
