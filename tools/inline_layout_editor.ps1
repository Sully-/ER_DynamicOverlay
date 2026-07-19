# Inline the layout editor into a single self-contained HTML file.
#
# Why: shipping standalone .js files makes automated malware scanners (e.g. Nexus)
# execute them with Windows Script Host (wscript.exe) from %TEMP%, which trips the
# Sigma rule "Script Interpreter Execution From Suspicious Folder" (false positive).
# Folding the CSS/JS into the HTML means the release archive contains no .js files.
#
# Usage:
#   .\tools\inline_layout_editor.ps1 -OutFile path\to\layout_editor.html
#   .\tools\inline_layout_editor.ps1 -SourceDir tools/layout_editor -OutFile out/layout_editor.html

param(
    [string]$SourceDir,
    [Parameter(Mandatory = $true)][string]$OutFile
)

$ErrorActionPreference = "Stop"

if (-not $SourceDir) { $SourceDir = Join-Path $PSScriptRoot "layout_editor" }

$htmlPath   = Join-Path $SourceDir "layout_editor.html"
$assetsDir  = Join-Path $SourceDir "layout_editor_assets"

if (-not (Test-Path $htmlPath))  { throw "Not found: $htmlPath" }
if (-not (Test-Path $assetsDir)) { throw "Not found: $assetsDir" }

$html = Get-Content -Raw -Encoding UTF8 $htmlPath

function Read-Asset([string]$name) {
    $p = Join-Path $assetsDir $name
    if (-not (Test-Path $p)) { throw "Not found: $p" }
    $content = Get-Content -Raw -Encoding UTF8 $p
    # A literal </script> inside inlined JS would terminate the <script> block early.
    if ($content -match '</script') {
        throw "Asset '$name' contains a literal '</script>' which cannot be safely inlined."
    }
    return $content
}

# Inline the stylesheet: <link rel="stylesheet" href="layout_editor_assets/style.css" />
$css = Get-Content -Raw -Encoding UTF8 (Join-Path $assetsDir "style.css")
$linkPattern = '<link\s+rel="stylesheet"\s+href="layout_editor_assets/style\.css"\s*/?>'
if ($html -notmatch $linkPattern) { throw "Could not find the stylesheet <link> tag to inline." }
$html = [regex]::Replace($html, $linkPattern, { "<style>`n$css`n</style>" })

# Inline each script: <script src="layout_editor_assets/NAME.js"></script>
$scriptPattern = '<script\s+src="layout_editor_assets/([^"]+\.js)"\s*></script>'
$html = [regex]::Replace($html, $scriptPattern, {
    param($m)
    $name = $m.Groups[1].Value
    $js = Read-Asset $name
    "<script>`n$js`n</script>"
})

if ($html -match 'layout_editor_assets/') {
    throw "Some 'layout_editor_assets/' references remain after inlining - aborting."
}

$outDir = Split-Path -Parent $OutFile
if ($outDir -and -not (Test-Path $outDir)) { New-Item -ItemType Directory -Force $outDir | Out-Null }

# Resolve to an absolute path (the file may not exist yet).
if ([System.IO.Path]::IsPathRooted($OutFile)) {
    $absOut = $OutFile
} else {
    $absOut = Join-Path (Get-Location).Path $OutFile
}

# Write UTF-8 without BOM.
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($absOut, $html, $utf8NoBom)

Write-Host "Inlined layout editor -> $OutFile"
