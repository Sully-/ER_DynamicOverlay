# Bundle a local release zip (same layout as .github/workflows/release.yml).
# Usage:
#   .\tools\bundle_release.ps1                  # build release + bundle
#   .\tools\bundle_release.ps1 -Version v0.1.0-test
#   .\tools\bundle_release.ps1 -SkipBuild       # reuse existing target/release/

param(
    [string]$Version = "local-test",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

if (-not $SkipBuild) {
    Write-Host "Building release..."
    cargo build --release

    Write-Host "Publishing companion..."
    dotnet publish companion/er_checks_extractor/er_checks_extractor.csproj -c Release -o target/release/companion
}

$releaseDir = Join-Path $root "target/release"
$required = @(
    "er_overlay_injector.exe",
    "er_overlay.dll",
    "er_overlay.toml",
    "layouts",
    "tables",
    "assets",
    "companion/er_checks_extractor.exe"
)
foreach ($item in $required) {
    if (-not (Test-Path (Join-Path $releaseDir $item))) {
        throw "Missing target/release/$item — run 'cargo build --release' first (or without -SkipBuild)."
    }
}

$out = "er-overlay-$Version"
$zip = "$out.zip"

if (Test-Path $out) { Remove-Item $out -Recurse -Force }
if (Test-Path $zip) { Remove-Item $zip -Force }

New-Item -ItemType Directory -Force $out | Out-Null

Copy-Item "$releaseDir/er_overlay_injector.exe" $out/
Copy-Item "$releaseDir/er_overlay.dll"          $out/
Copy-Item "$releaseDir/er_overlay.toml"         $out/
Copy-Item "$releaseDir/layouts" $out/ -Recurse
Copy-Item "$releaseDir/tables"  $out/ -Recurse
Copy-Item "$releaseDir/assets"  $out/ -Recurse
Copy-Item "$releaseDir/companion" $out/ -Recurse
Copy-Item README.md, LICENSE     $out/

Copy-Item tools/layout_editor/layout_editor.html $out/
Copy-Item tools/layout_editor/layout_editor_assets $out/layout_editor_assets -Recurse

Compress-Archive -Path "$out/*" -DestinationPath $zip -Force

Write-Host ""
Write-Host "Done: $zip"
Write-Host "Folder: $out/"
Write-Host "Test layout editor: $out/layout_editor.html"
