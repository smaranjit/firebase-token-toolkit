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

# Verify the CRT is statically linked before packaging. A GitHub runner has the
# Visual C++ Redistributable installed, so a dynamically linked binary runs fine
# here and fails only on a user's clean machine — exactly how v0.1.0 shipped
# broken. Checking the import by name is cheap and needs no VS dev environment.
Write-Host "==> checking the CRT is statically linked"
$ExePath = "target\$Target\release\$Name.exe"
$Bytes = [System.IO.File]::ReadAllBytes($ExePath)
$AsText = [System.Text.Encoding]::ASCII.GetString($Bytes)
if ($AsText -match 'VCRUNTIME\d+\.dll') {
    throw "$Name.exe imports $($Matches[0]) - the CRT is not statically linked. " +
          "Check the rustflags in .cargo/config.toml."
}
Write-Host "    no VCRUNTIME import; the runtime is baked in"

Write-Host "==> staging"
if (Test-Path "dist\stage-windows") { Remove-Item -Recurse -Force "dist\stage-windows" }
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item "target\$Target\release\$Name.exe" $Stage
# Read the document list out of common.sh rather than restating it. This script
# cannot source a shell file, and the copy that used to live here drifted the
# moment common.sh changed — v0.1.4 shipped a Windows archive still containing
# files the other two platforms had dropped.
$DocsLine = Select-String -Path scripts/common.sh -Pattern '^DOCS=\((.*)\)' | Select-Object -First 1
if (-not $DocsLine) { throw "could not find the DOCS list in scripts/common.sh" }
$Docs = $DocsLine.Matches.Groups[1].Value -split '\s+' | Where-Object { $_ }
if (-not $Docs) { throw "the DOCS list in scripts/common.sh parsed as empty" }
Write-Host "    shipping docs: $($Docs -join ', ')"
Copy-Item $Docs $Stage

Write-Host "==> packing $Out"
if (Test-Path $Out) { Remove-Item -Force $Out }
Compress-Archive -Path $Stage -DestinationPath $Out
Remove-Item -Recurse -Force "dist\stage-windows"

Write-Host "==> done"
Get-Item $Out
