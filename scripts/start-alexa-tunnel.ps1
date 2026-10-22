param([switch]$DownloadOnly)
$ErrorActionPreference = 'Stop'
# Development dependency, never installed as a system service. Official release
# metadata: https://github.com/cloudflare/cloudflared/releases/tag/2026.10.3
$alexaTunnelDirectory = Join-Path $PSScriptRoot '..\target\tools'
$alexaTunnelBinary = Join-Path $alexaTunnelDirectory 'cloudflared-2026.10.3.exe'
$alexaTunnelDigest = '76b2805253d0d6ffacb092a7acfbee99e7af6917835c0c5410e23f3ef5052da2'
New-Item -ItemType Directory -Force $alexaTunnelDirectory | Out-Null
if (-not (Test-Path -LiteralPath $alexaTunnelBinary)) {
    $alexaTunnelDownload = Join-Path $alexaTunnelDirectory 'cloudflared-2026.10.3.download'
    Invoke-WebRequest 'https://github.com/cloudflare/cloudflared/releases/download/2026.10.3/cloudflared-windows-amd64.exe' -OutFile $alexaTunnelDownload
    if ((Get-FileHash -LiteralPath $alexaTunnelDownload -Algorithm SHA256).Hash.ToLowerInvariant() -ne $alexaTunnelDigest) {
        throw 'cloudflared SHA-256 mismatch / Empreinte cloudflared incorrecte'
    }
    Move-Item -LiteralPath $alexaTunnelDownload -Destination $alexaTunnelBinary
}
if ((Get-FileHash -LiteralPath $alexaTunnelBinary -Algorithm SHA256).Hash.ToLowerInvariant() -ne $alexaTunnelDigest) {
    throw 'cloudflared SHA-256 mismatch / Empreinte cloudflared incorrecte'
}
if ($DownloadOnly) {
    Write-Output 'cloudflared 2026.10.3 ready (SHA-256 verified) / prêt (SHA-256 vérifié)'
    return
}
# Expose only the media gateway. Never point a tunnel at the login proxy (8767).
& $alexaTunnelBinary --no-autoupdate tunnel --url http://127.0.0.1:8766
if ($LASTEXITCODE -ne 0) { throw 'Tunnel stopped with an error / Arrêt du tunnel avec erreur' }
