$Url = "https://github.com/emredm1821/cat/releases/download/latest/cat.exe"
$Dest = Join-Path (Get-Location) "cat.exe"

Invoke-WebRequest -Uri $Url -OutFile $Dest

if (Test-Path $Dest) {
    Write-Host "Successfully installed cat into your current directory." -ForegroundColor Green
} else {
    Write-Host "Failed to install cat." -ForegroundColor Red
    exit 1
}
