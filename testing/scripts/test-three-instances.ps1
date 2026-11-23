# Starts one cargo tauri dev instance and two debug executable instances for NTO.
# Run from any directory: .\testing\scripts\test-three-instances.ps1

param(
    [switch]$StartXmpp,
    [switch]$StopXmpp,
    [switch]$KeepDevRunning
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$debugExe = Join-Path $projectRoot "target\debug\nto.exe"
$xmppScript = Join-Path $projectRoot "testing\xmpp-server\start-xmpp.ps1"

Set-Location $projectRoot

if ($StopXmpp) {
    & $xmppScript -Stop
    exit $LASTEXITCODE
}

Write-Host "NTO three-instance test launcher" -ForegroundColor Cyan

if ($StartXmpp) {
    & $xmppScript
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to start ejabberd."
    }
}

if (-not (Test-Path $debugExe)) {
    Write-Host "Debug executable not found. Building it..." -ForegroundColor Yellow
    cargo build --manifest-path (Join-Path $projectRoot "src-tauri\Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed."
    }
}

if (-not (Test-Path $debugExe)) {
    throw "Could not find target\debug\nto.exe."
}

$existingDev = Get-Process -Name trunk -ErrorAction SilentlyContinue
if ($existingDev) {
    throw "A Trunk instance is already running on port 1420. Stop it before launching a new Tauri dev instance."
}

$env:NTO_XMPP_HOST = "localhost"
$env:NTO_XMPP_PORT = "5223"
$env:NTO_XMPP_ALLOW_INVALID_CERTS = "1"

Write-Host "Starting instance 1: cargo tauri dev" -ForegroundColor Green
$devProcess = Start-Process -FilePath "cargo" `
    -ArgumentList "tauri dev -- -- --username testuser --password testpassword" `
    -WorkingDirectory $projectRoot `
    -PassThru

Write-Host "Waiting for the Tauri dev server..." -ForegroundColor Yellow
$ready = $false
for ($i = 0; $i -lt 60; $i++) {
    try {
        $listener = Get-NetTCPConnection -State Listen -LocalPort 1420 -ErrorAction SilentlyContinue
        if ($listener) {
            $ready = $true
            break
        }
    } catch {}
    Start-Sleep -Milliseconds 500
}

if (-not $ready) {
    if (-not $devProcess.HasExited) {
        Stop-Process -Id $devProcess.Id -Force
    }
    throw "cargo tauri dev did not open port 1420 in time."
}

Start-Sleep -Seconds 2

Write-Host "Starting instance 2: target\debug\nto.exe" -ForegroundColor Green
Start-Process -FilePath $debugExe `
    -ArgumentList "--username kampelo --password password123" `
    -WorkingDirectory $projectRoot

Write-Host "Starting instance 3: target\debug\nto.exe" -ForegroundColor Green
Start-Process -FilePath $debugExe `
    -ArgumentList "--username ivokds --password password123" `
    -WorkingDirectory $projectRoot

Write-Host ""
Write-Host "Three instances started." -ForegroundColor Cyan
Write-Host "1. cargo tauri dev"
Write-Host "2. target\debug\nto.exe"
Write-Host "3. target\debug\nto.exe"
Write-Host "Instance accounts:"
Write-Host "  1. testuser@localhost / testpassword"
Write-Host "  2. kampelo@localhost / password123"
Write-Host "  3. ivokds@localhost / password123"
Write-Host ""
Write-Host "Test accounts:"
Write-Host "  testuser@localhost / testpassword"
Write-Host "  kampelo@localhost / password123"
Write-Host "  ivokds@localhost / password123"
Write-Host ""
Write-Host "Press Ctrl+C in the cargo tauri dev terminal to stop that instance."

if ($KeepDevRunning) {
    Wait-Process -Id $devProcess.Id
} else {
    Write-Host "The cargo tauri dev process will remain in a separate window."
}
