# Zips the 12 background *.mp4 files and republishes them to the media-v1
# GitHub release (not git-tracked; the app only ever downloads from there,
# see src-tauri/src/commands/media.rs). Requires `gh` authenticated.
param(
    [Parameter(Mandatory)]
    [int]$Version,
    [string]$VideosDir = "."
)

$ErrorActionPreference = "Stop"
$zipName = "strata-media.zip"
$zipPath = Join-Path $env:TEMP $zipName

Compress-Archive -Path (Join-Path $VideosDir "*.mp4") -DestinationPath $zipPath -Force
$hash = (Get-FileHash -Algorithm SHA256 $zipPath).Hash.ToLower()

$manifest = @{
    version = $Version
    url     = "https://github.com/nihalantiir/stratalauncher/releases/download/media-v1/$zipName"
    sha256  = $hash
}
$manifestPath = Join-Path $env:TEMP "media.json"
$manifest | ConvertTo-Json -Depth 5 | Set-Content -Path $manifestPath -Encoding utf8

if (-not (gh release view media-v1 2>$null)) {
    gh release create media-v1 --title "media-v1" --notes "Background videos for the launcher UI, downloaded on first launch." --latest=false
}
gh release upload media-v1 $zipPath $manifestPath --clobber

Write-Host "Published media-v1 version $Version"
