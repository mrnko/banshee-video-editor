$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$version = (Get-Content -Raw (Join-Path $projectRoot 'VERSION')).Trim()
$releaseDir = Join-Path $projectRoot 'target\release'
$stageDir = Join-Path $projectRoot "dist\Banshee-Video-Editor-$version-portable"
$resolvedStage = [IO.Path]::GetFullPath($stageDir)
$resolvedRoot = [IO.Path]::GetFullPath($projectRoot)
if (-not $resolvedStage.StartsWith($resolvedRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe staging path.' }
if (Test-Path -LiteralPath $resolvedStage) { Remove-Item -LiteralPath $resolvedStage -Recurse -Force }
New-Item -ItemType Directory -Path $resolvedStage | Out-Null
Copy-Item -LiteralPath (Join-Path $releaseDir 'banshee-video-editor.exe') -Destination $resolvedStage
Copy-Item -LiteralPath (Join-Path $projectRoot 'LICENSE') -Destination $resolvedStage
Copy-Item -LiteralPath (Join-Path $projectRoot 'THIRD_PARTY_NOTICES.md') -Destination $resolvedStage
New-Item -ItemType File -Path (Join-Path $resolvedStage 'banshee-portable.marker') -Force | Out-Null
$toolDir = Join-Path $projectRoot 'tools\bin'
if (Test-Path -LiteralPath $toolDir) { Copy-Item -LiteralPath $toolDir -Destination (Join-Path $resolvedStage 'tools') -Recurse }
Compress-Archive -Path "$resolvedStage\*" -DestinationPath "$resolvedStage.zip" -Force
Write-Output "$resolvedStage.zip"
