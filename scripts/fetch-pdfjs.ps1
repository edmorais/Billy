# Downloads the pinned PDF.js release (Mozilla's PDF viewer, as used in Firefox) and puts its
# prebuilt viewer in static/pdfjs, where the app serves it from /pdfjs/web/viewer.html.
# The archive is checked against a SHA-256 hash before use. Source maps are left out.

$ErrorActionPreference = 'Stop'

$Version = '6.5.438'
$Sha256 = 'fcbe9ba21d9911843e3502adc178a7f549a524a4055f887bedc942e5490c6ba1'
$Url = "https://github.com/mozilla/pdf.js/releases/download/v$Version/pdfjs-$Version-dist.zip"

$root = Split-Path -Parent $PSScriptRoot
$target = Join-Path $root 'static\pdfjs'
$stamp = Join-Path $target 'pdfjs.version'

if ((Test-Path $stamp) -and ((Get-Content $stamp -Raw).Trim() -eq $Version)) {
    Write-Host "PDF.js $Version is already in place."
    exit 0
}

$work = Join-Path ([System.IO.Path]::GetTempPath()) "billy-pdfjs-$Version-$PID"
New-Item -ItemType Directory -Force $work | Out-Null
try {
    $archive = Join-Path $work 'pdfjs.zip'
    Write-Host "Downloading PDF.js $Version..."
    Invoke-WebRequest -Uri $Url -OutFile $archive -UseBasicParsing

    $actual = (Get-FileHash $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $Sha256) {
        throw "PDF.js download has the wrong SHA-256 hash: $actual"
    }

    $extracted = Join-Path $work 'x'
    Expand-Archive -Path $archive -DestinationPath $extracted
    Get-ChildItem $extracted -Recurse -Filter *.map | Remove-Item -Force
    # The sample document is not needed.
    Get-ChildItem $extracted -Recurse -Filter *.pdf | Remove-Item -Force

    if (Test-Path $target) { Remove-Item -Recurse -Force $target }
    New-Item -ItemType Directory -Force $target | Out-Null
    Copy-Item (Join-Path $extracted '*') $target -Recurse -Force
    Set-Content -Path $stamp -Value $Version -Encoding ascii
    Write-Host "PDF.js $Version installed in $target"
}
finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
