# Adapted from Oxyde-GPUI (Carlton2001, GPL-3.0). Signed installer manifest.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version,
    [Parameter(Mandatory)][string]$Destination,
    [Parameter(Mandatory)][ValidatePattern('^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$')][string]$Repository,
    [Parameter(Mandatory)][string]$Setup,
    [Parameter(Mandatory)][string]$LinuxArchive
)
$ErrorActionPreference = 'Stop'
function Get-SignedAsset([string]$Path, [string]$ExpectedName) {
    if ([IO.Path]::GetFileName($Path) -cne $ExpectedName -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw "Missing or unexpected release asset $ExpectedName." }
    $signature = (Get-Content -LiteralPath "$Path.sig" -Raw).Trim()
    if (-not $signature) { throw "Missing signature for $ExpectedName." }
    @{
        url = "https://github.com/$Repository/releases/download/v$Version/$ExpectedName"
        signature = $signature
    }
}
$manifest = [ordered]@{
    version = $Version
    pub_date = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ss.fffZ')
    platforms = @{
        'windows-x86_64' = Get-SignedAsset $Setup "Soliton_${Version}_x64-setup.exe"
        'linux-x86_64' = Get-SignedAsset $LinuxArchive "Soliton-$Version-x86_64-linux.tar.gz"
    }
}
[IO.File]::WriteAllText($ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Destination), ($manifest | ConvertTo-Json -Depth 4) + "`n", [Text.UTF8Encoding]::new($false))
