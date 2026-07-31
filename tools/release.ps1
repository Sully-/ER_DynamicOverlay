# Cut a new release: bump the workspace version, commit, tag and push.
# The pushed tag triggers .github/workflows/release.yml, which builds and
# publishes the GitHub Release. The auto-updater in er_overlay_injector then
# picks it up on the users' machines.
#
# Dirty working trees are fine: tracked WIP is stashed around the release
# commit so you can tag HEAD without committing in-progress work.
#
# Usage:
#   .\tools\release.ps1 -Version 1.3.0
#   .\tools\release.ps1 -Version v1.3.0    # leading v is accepted

param(
    [Parameter(Mandatory = $true)]
    [string]$Version
)

$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $root

# Normalize: X.Y.Z for Cargo, vX.Y.Z for the git tag.
$semver = $Version.TrimStart('v')
if ($semver -notmatch '^\d+\.\d+\.\d+(?:[-+].+)?$') {
    throw "Version '$Version' is not valid semver (expected X.Y.Z, optionally with -pre/+build)."
}
$tag = "v$semver"

# Make sure the tag does not already exist.
$existing = git tag --list $tag
if ($existing) {
    throw "Tag $tag already exists."
}

# Stash tracked WIP so the release commit only contains the version bump.
# Untracked files are left alone (they don't affect HEAD / the release commit).
$stashed = $false
$trackedDirty = git status --porcelain -uno
if ($trackedDirty) {
    Write-Host "Working tree has uncommitted changes; stashing them for the release..."
    Write-Host $trackedDirty
    git stash push -m "release.ps1 auto-stash before $tag"
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to stash working tree changes."
    }
    $stashed = $true
}

try {
    $cargoToml = Join-Path $root "Cargo.toml"
    $content = Get-Content $cargoToml -Raw

    # Replace the version line inside [workspace.package] only.
    $pattern = '(?ms)(\[workspace\.package\].*?\bversion\s*=\s*")[^"]*(")'
    if ($content -notmatch $pattern) {
        throw "Could not find [workspace.package] version in Cargo.toml."
    }
    $updated = [regex]::Replace($content, $pattern, "`${1}$semver`${2}", 1)
    if ($updated -eq $content) {
        Write-Host "Cargo.toml already at version $semver."
    } else {
        Set-Content -Path $cargoToml -Value $updated -NoNewline
        Write-Host "Set [workspace.package] version = $semver"
    }

    # Refresh Cargo.lock so the workspace crates reflect the new version.
    Write-Host "Updating Cargo.lock ..."
    cargo update --workspace --offline 2>$null
    if ($LASTEXITCODE -ne 0) { cargo update --workspace }

    git add Cargo.toml Cargo.lock
    git commit -m "release: $tag"
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to create release commit (nothing to commit?)."
    }
    git tag $tag
    git push origin HEAD --tags
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to push release commit/tag."
    }

    Write-Host ""
    Write-Host "Released $tag."
    Write-Host "GitHub Actions will build and publish the release zip."
    Write-Host "Users get the update prompt next time they run er_overlay_injector.exe."
}
finally {
    if ($stashed) {
        Write-Host ""
        Write-Host "Restoring stashed WIP..."
        git stash pop
        if ($LASTEXITCODE -ne 0) {
            Write-Warning "stash pop had conflicts. Resolve them, then drop the stash if needed (git stash list)."
        }
    }
}
