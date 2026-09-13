# Build the Windows release artifact natively:
#   dist\<name>-v<version>-x86_64-windows.zip
#
# For contributors on Windows. Requires the MSVC toolchain.
# Mirrors the build-windows job in .github\workflows\release.yml.
$ErrorActionPreference = "Stop"

Set-Location (Join-Path $PSScriptRoot "..")

$Name = "firebase-token-toolkit"
$Target = "x86_64-pc-windows-msvc"

# Read the version from Cargo.toml so the artifact name can't drift from the build.
$Version = (Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]*)"' |
    Select-Object -First 1).Matches.Groups[1].Value

$Stage = "dist\stage-windows\$Name-v$Version-x86_64-windows"
$Out = "dist\$Name-v$Version-x86_64-windows.zip"

Write-Host "==> building $Name v$Version for $Target"
cargo build --release --target $Target
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

Write-Host "==> staging"
if (Test-Path "dist\stage-windows") { Remove-Item -Recurse -Force "dist\stage-windows" }
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item "target\$Target\release\$Name.exe" $Stage
Copy-Item README.md, LICENSE, CHANGELOG.md, SECURITY.md $Stage

Write-Host "==> packing $Out"
if (Test-Path $Out) { Remove-Item -Force $Out }
Compress-Archive -Path $Stage -DestinationPath $Out
Remove-Item -Recurse -Force "dist\stage-windows"

Write-Host "==> done"
Get-Item $Out
