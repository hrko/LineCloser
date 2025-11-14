# release.ps1

# Strict mode
Set-StrictMode -Version Latest

# 1. Fetch latest tags
git fetch --tags

# 2. Get latest tag
$latestTag = git tag --sort=-v:refname | Select-Object -First 1
if (-not $latestTag) {
    $latestTag = "v0.0.0"
}
$latestVersion = $latestTag.TrimStart('v')
$versionParts = $latestVersion.Split('.')
$major = [int]$versionParts[0]
$minor = [int]$versionParts[1]
$patch = [int]$versionParts[2]

Write-Host "Current version: v$major.$minor.$patch"

# 3. Prompt for version bump
$choice = Read-Host "Which part to increment? (major, minor, patch) [default: patch]"
if ([string]::IsNullOrWhiteSpace($choice)) {
    $choice = "patch"
}

# 4. Calculate new version
switch ($choice) {
    "major" { $major++ ; $minor = 0 ; $patch = 0 }
    "minor" { $minor++ ; $patch = 0 }
    "patch" { $patch++ }
    default { Write-Error "Invalid choice: $choice"; exit 1 }
}
$newVersion = "$major.$minor.$patch"
$newTag = "v$newVersion"
Write-Host "New version: $newTag"

# 5. Update Cargo.toml
$cargoTomlPath = "./Cargo.toml"
$lines = Get-Content $cargoTomlPath
$newLines = @()
$updated = $false
foreach ($line in $lines) {
    if (-not $updated -and $line -match '^\s*version\s*=\s*') {
        $newLines += 'version = "' + $newVersion + '"'
        $updated = $true
    } else {
        $newLines += $line
    }
}
Set-Content -Path $cargoTomlPath -Value $newLines -NoNewline

# 6. Commit the change
git add $cargoTomlPath
git commit -m "chore(release): bump version to $newTag"

# 7. Tag the new commit
git tag -a $newTag -m "$newTag"

# 8. Push commit and tag
Write-Host "Pushing commit and tag to remote..."
git push
git push --tags

# 9. Create GitHub Release
Write-Host "Creating GitHub release..."
gh release create $newTag --generate-notes

Write-Host "Release $newTag created successfully."
