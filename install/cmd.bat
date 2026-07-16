@echo off
setlocal

set "URL=https://github.com/emredm1821/cat/releases/download/latest/cat.exe"
set "DEST=%CD%\cat.exe"

powershell -NoProfile -Command "Invoke-WebRequest -Uri '%URL%' -OutFile '%DEST%'"

if exist "%DEST%" (
    powershell -NoProfile -Command "Write-Host 'Successfully installed cat into your current directory.' -ForegroundColor Green"
) else (
    powershell -NoProfile -Command "Write-Host 'Failed to install cat.' -ForegroundColor Red"
    exit /b 1
)

endlocal
