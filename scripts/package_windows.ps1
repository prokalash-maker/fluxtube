param(
    [string]$Version = "0.1.0",
    [string]$Target = ""
)

$ErrorActionPreference = "Stop"

Write-Host "Building FluxTube release binary for Windows x64..."
if ([string]::IsNullOrWhiteSpace($Target)) {
    cargo build --release -p fluxtube-desktop
    $exePath = "target\release\fluxtube.exe"
} else {
    cargo build --release -p fluxtube-desktop --target $Target
    $exePath = "target\$Target\release\fluxtube.exe"
}

$distDir = "dist\windows\FluxTube-$Version-portable-x64"
if (Test-Path $distDir) { Remove-Item -Recurse -Force $distDir }
New-Item -ItemType Directory -Force -Path $distDir | Out-Null

Copy-Item $exePath -Destination "$distDir\fluxtube.exe"
Set-Content -Path "$distDir\portable.flag" -Value "FluxTube Portable Mode Enabled (stores local database, logs and cache in .\data\)" -Encoding UTF8
Copy-Item ".env.example" -Destination "$distDir\.env.example"
Copy-Item "GOOGLE_OAUTH_SETUP.md" -Destination "$distDir\GOOGLE_OAUTH_SETUP.md"
Copy-Item "README.md" -Destination "$distDir\README.md"
Copy-Item "LICENSE" -Destination "$distDir\LICENSE"

$zipPath = "dist\windows\FluxTube-$Version-portable-x64.zip"
if (Test-Path $zipPath) { Remove-Item -Force $zipPath }
Compress-Archive -Path "$distDir\*" -DestinationPath $zipPath

Write-Host "Portable bundle created: $zipPath"
