# Builds the Microsoft Store package (MSIX) of كلمة from a release build.
#
#   pwsh packaging/windows/msix/make-msix.ps1 -Version 1.24.0 -Out out
#
# Needs target/release/wordcraft.exe and wordcraft-cli.exe (cargo build --release) and the Windows
# SDK (makeappx.exe; the GitHub windows runners have it). The package is unsigned: the Store signs
# it on upload. To try it on your own PC first, sign a copy with a test certificate whose subject
# is the manifest's Publisher (see packaging/windows/msix/README.md).
#
# The tile images are drawn at build time from assets/app-icon/wordcraft-1024.png, so no new image
# files live in the repository.
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [string]$Out = "out"
)
$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "../../..")
$Stage = Join-Path $Root "target/msix-stage"
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
New-Item -ItemType Directory -Force -Path (Join-Path $Stage "Assets") | Out-Null
New-Item -ItemType Directory -Force -Path $Out | Out-Null

# MSIX versions have four parts, and Store packages keep the last at 0.
$parts = $Version.Split(".")
while ($parts.Count -lt 3) { $parts += "0" }
$msixVersion = "$($parts[0]).$($parts[1]).$($parts[2]).0"

Copy-Item (Join-Path $Root "target/release/wordcraft.exe") (Join-Path $Stage "Kalima.exe")
Copy-Item (Join-Path $Root "target/release/wordcraft-cli.exe") (Join-Path $Stage "kalima-cli.exe")
foreach ($f in @("LICENSE-MIT", "LICENSE-APACHE", "NOTICE", "ATTRIBUTION.md")) {
    Copy-Item (Join-Path $Root $f) $Stage
}
Get-ChildItem (Join-Path $Root "assets/fonts") -Filter "OFL-*.txt" | Copy-Item -Destination $Stage

$manifest = Get-Content -Raw -Encoding UTF8 (Join-Path $PSScriptRoot "AppxManifest.xml.in")
$manifest = $manifest.Replace("@VERSION@", $msixVersion)
[System.IO.File]::WriteAllText((Join-Path $Stage "AppxManifest.xml"), $manifest, (New-Object System.Text.UTF8Encoding $false))

# Tiles: the app icon scaled onto a transparent canvas of each size (wide tiles keep it centred).
Add-Type -AssemblyName System.Drawing
$icon = [System.Drawing.Image]::FromFile((Join-Path $Root "assets/app-icon/wordcraft-1024.png"))
function Save-Tile([string]$name, [int]$w, [int]$h, [double]$scale) {
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.Clear([System.Drawing.Color]::Transparent)
    $side = [int]([Math]::Min($w, $h) * $scale)
    $g.DrawImage($icon, [int](($w - $side) / 2), [int](($h - $side) / 2), $side, $side)
    $bmp.Save((Join-Path $Stage "Assets/$name"), [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
}
Save-Tile "StoreLogo.png" 50 50 1.0
Save-Tile "Square44x44Logo.png" 44 44 1.0
Save-Tile "Square44x44Logo.targetsize-256_altform-unplated.png" 256 256 1.0
Save-Tile "Square150x150Logo.png" 150 150 0.66
Save-Tile "Wide310x150Logo.png" 310 150 0.66
$icon.Dispose()

$sdk = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\makeappx.exe" | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $sdk) { throw "makeappx.exe not found: install the Windows SDK." }
$msix = Join-Path (Resolve-Path $Out) "kalima-$Version-windows-x64-store.msix"
& $sdk.FullName pack /o /d $Stage /p $msix
if ($LASTEXITCODE -ne 0) { throw "makeappx failed ($LASTEXITCODE)" }
Write-Host "Wrote $msix"
