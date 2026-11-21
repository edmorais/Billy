# Downloads the pinned PDFium build for Windows x64 and puts pdfium.dll in src-tauri/resources.
# The binary comes from https://github.com/bblanchon/pdfium-binaries and is checked against a
# SHA-256 hash before use. Keep the version in step with the pdfium-render crate's API version.

$ErrorActionPreference = 'Stop'

$Version = '8125'
$Sha256 = '9a964261760cff52570aabf038ab346d3a5034dd8b30d0d5fe8861235d356b8b'
$Url = "https://github.com/bblanchon/pdfium-binaries/releases/download/chromium/$Version/pdfium-win-x64.tgz"

$root = Split-Path -Parent $PSScriptRoot
$resources = Join-Path $root 'src-tauri\resources'
$dll = Join-Path $resources 'pdfium.dll'
$stamp = Join-Path $resources 'pdfium.version'

if ((Test-Path $dll) -and (Test-Path $stamp) -and ((Get-Content $stamp -Raw).Trim() -eq $Version)) {
    Write-Host "PDFium $Version is already in place."
    exit 0
}

$work = Join-Path ([System.IO.Path]::GetTempPath()) "billy-pdfium-$Version-$PID"
New-Item -ItemType Directory -Force $work | Out-Null
try {
    $archive = Join-Path $work 'pdfium.tgz'
    Write-Host "Downloading PDFium $Version..."
    Invoke-WebRequest -Uri $Url -OutFile $archive -UseBasicParsing

    $actual = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $Sha256) {
        throw "PDFium download has the wrong SHA-256 hash: $actual"
    }

    tar -xzf $archive -C $work
    if ($LASTEXITCODE -ne 0) { throw 'Could not extract the PDFium archive.' }

    New-Item -ItemType Directory -Force $resources | Out-Null
    Copy-Item (Join-Path $work 'bin\pdfium.dll') $dll -Force
    Copy-Item (Join-Path $work 'LICENSE') (Join-Path $resources 'pdfium-LICENSE.txt') -Force
    Set-Content -Path $stamp -Value $Version -Encoding ascii
    Write-Host "PDFium $Version installed in $resources"
}
finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
