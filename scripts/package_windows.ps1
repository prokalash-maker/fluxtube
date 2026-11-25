param(
    [string]$Version = "0.1.0"
)

$ErrorActionPreference = "Stop"

Write-Host "Building FluxTube release binary for Windows x64..."
cargo build --release -p fluxtube-desktop

$distDir = "dist\windows\FluxTube-$Version-windows-x64"
if (Test-Path $distDir) { Remove-Item -Recurse -Force $distDir }
New-Item -ItemType Directory -Force -Path $distDir | Out-Null

Copy-Item "target\release\fluxtube.exe" -Destination "$distDir\fluxtube.exe"
Copy-Item ".env.example" -Destination "$distDir\.env.example"
Copy-Item "README.md" -Destination "$distDir\README.md"
Copy-Item "LICENSE" -Destination "$distDir\LICENSE"

$zipPath = "dist\windows\FluxTube-$Version-portable-x64.zip"
if (Test-Path $zipPath) { Remove-Item -Force $zipPath }
Compress-Archive -Path "$distDir\*" -DestinationPath $zipPath

Write-Host "Portable bundle created: $zipPath"
