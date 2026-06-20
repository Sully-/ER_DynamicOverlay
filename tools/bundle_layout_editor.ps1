# Quick bundle to test only the layout editor (no Rust build).
# Usage: .\tools\bundle_layout_editor.ps1

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

$out = "layout-editor-test"
if (Test-Path $out) { Remove-Item $out -Recurse -Force }

New-Item -ItemType Directory -Force $out | Out-Null

Copy-Item tools/layout_editor/layout_editor.html $out/
Copy-Item tools/layout_editor/layout_editor_assets $out/layout_editor_assets -Recurse

if (Test-Path assets/icons) {
    Copy-Item assets/icons $out/assets/icons -Recurse
} else {
    Write-Warning "assets/icons not found at repo root - icons will be missing in the test bundle."
}

Write-Host ""
Write-Host "Done: $out/"
Write-Host "Open: $out/layout_editor.html"
