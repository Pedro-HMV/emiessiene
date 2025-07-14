# NTO XMPP Development Starter
# This script manages the WSL Prosody server and starts the application

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("start", "stop", "restart", "status", "logs", "users")]
    [string]$Action = "start",
    
    [switch]$ShowLogs,
    [switch]$Force
)

Write-Host "🚀 NTO XMPP Development Manager" -ForegroundColor Green
Write-Host "═══════════════════════════════════════" -ForegroundColor Green

$ErrorActionPreference = "Stop"

# Function to check if WSL is available
function Test-WSL {
    try {
        $wslVersion = wsl --status 2>&1
        if ($LASTEXITCODE -ne 0) {
            throw "WSL not properly configured"
        }
        return $true
    }
    catch {
        Write-Host "❌ WSL is not available or not configured" -ForegroundColor Red
        Write-Host "💡 Install WSL with: wsl --install -d Ubuntu" -ForegroundColor Yellow
        return $false
    }
}

# Function to check if Prosody is installed in WSL
function Test-ProsodyInstalled {
    try {
        $result = wsl which prosody 2>$null
        return $LASTEXITCODE -eq 0
    }
    catch {
        return $false
    }
}

# Function to get Prosody status
function Get-ProsodyStatus {
    try {
        $status = wsl sudo systemctl is-active prosody 2>$null
        return $status.Trim()
    }
    catch {
        return "unknown"
    }
}

# Function to test XMPP port connectivity
function Test-XMPPPort {
    $connection = Test-NetConnection -ComputerName localhost -Port 5222 -WarningAction SilentlyContinue
    return $connection.TcpTestSucceeded
}

# Function to start Prosody
function Start-Prosody {
    Write-Host "📡 Starting Prosody XMPP server..." -ForegroundColor Cyan
    
    try {
        wsl sudo systemctl start prosody
        Start-Sleep -Seconds 3
        
        $status = Get-ProsodyStatus
        if ($status -eq "active") {
            Write-Host "✅ Prosody server started successfully" -ForegroundColor Green
            
            # Test port accessibility
            if (Test-XMPPPort) {
                Write-Host "✅ XMPP port 5222 is accessible from Windows" -ForegroundColor Green
                return $true
            } else {
                Write-Host "⚠️  Prosody is running but port 5222 is not accessible" -ForegroundColor Yellow
                Write-Host "💡 Try restarting WSL: wsl --shutdown" -ForegroundColor Yellow
                return $false
            }
        } else {
            Write-Host "❌ Failed to start Prosody (status: $status)" -ForegroundColor Red
            return $false
        }
    }
    catch {
        Write-Host "❌ Error starting Prosody: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Function to stop Prosody
function Stop-Prosody {
    Write-Host "🛑 Stopping Prosody XMPP server..." -ForegroundColor Cyan
    
    try {
        wsl sudo systemctl stop prosody
        Start-Sleep -Seconds 2
        
        $status = Get-ProsodyStatus
        if ($status -eq "inactive") {
            Write-Host "✅ Prosody server stopped successfully" -ForegroundColor Green
            return $true
        } else {
            Write-Host "⚠️  Prosody may still be running (status: $status)" -ForegroundColor Yellow
            return $false
        }
    }
    catch {
        Write-Host "❌ Error stopping Prosody: $($_.Exception.Message)" -ForegroundColor Red
        return $false
    }
}

# Function to restart Prosody
function Restart-Prosody {
    Write-Host "🔄 Restarting Prosody XMPP server..." -ForegroundColor Cyan
    
    Stop-Prosody | Out-Null
    Start-Sleep -Seconds 2
    return Start-Prosody
}

# Function to show Prosody status
function Show-ProsodyStatus {
    Write-Host "📊 Checking Prosody status..." -ForegroundColor Cyan
    
    $status = Get-ProsodyStatus
    $portOpen = Test-XMPPPort
    
    Write-Host "Service Status: " -NoNewline
    switch ($status) {
        "active" { Write-Host "RUNNING ✅" -ForegroundColor Green }
        "inactive" { Write-Host "STOPPED 🛑" -ForegroundColor Red }
        "failed" { Write-Host "FAILED ❌" -ForegroundColor Red }
        default { Write-Host "UNKNOWN ❓" -ForegroundColor Yellow }
    }
    
    Write-Host "Port 5222: " -NoNewline
    if ($portOpen) {
        Write-Host "ACCESSIBLE ✅" -ForegroundColor Green
    } else {
        Write-Host "NOT ACCESSIBLE ❌" -ForegroundColor Red
    }
    
    # Show recent log entries
    Write-Host "`n📄 Recent log entries:" -ForegroundColor Cyan
    try {
        wsl sudo tail -n 5 /var/log/prosody/prosody.log 2>$null
    }
    catch {
        Write-Host "Could not read log files" -ForegroundColor Yellow
    }
}

# Function to show live logs
function Show-ProsodyLogs {
    Write-Host "📄 Showing Prosody logs (Press Ctrl+C to stop)..." -ForegroundColor Cyan
    Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Gray
    
    try {
        wsl sudo tail -f /var/log/prosody/prosody.log
    }
    catch {
        Write-Host "❌ Could not access log files" -ForegroundColor Red
    }
}

# Function to manage users
function Show-Users {
    Write-Host "👥 XMPP Users:" -ForegroundColor Cyan
    
    try {
        # List users (this might not work with all Prosody versions)
        Write-Host "Attempting to list users..." -ForegroundColor Gray
        wsl sudo prosodyctl list_users nto.local 2>$null
        
        if ($LASTEXITCODE -ne 0) {
            Write-Host "💡 User listing not available. Create test users with:" -ForegroundColor Yellow
            Write-Host "   wsl sudo prosodyctl adduser alice@nto.local" -ForegroundColor Gray
            Write-Host "   wsl sudo prosodyctl adduser bob@nto.local" -ForegroundColor Gray
        }
    }
    catch {
        Write-Host "❌ Could not list users" -ForegroundColor Red
    }
}

# Function to start NTO development
function Start-Development {
    Write-Host "🎮 Starting NTO development server..." -ForegroundColor Cyan
    Write-Host "This will start both frontend and backend with hot reload" -ForegroundColor Gray
    Write-Host "Press Ctrl+C to stop the development server" -ForegroundColor Gray
    Write-Host "═══════════════════════════════════════════════════" -ForegroundColor Gray
    
    try {
        cargo tauri dev
    }
    catch {
        Write-Host "❌ Failed to start development server" -ForegroundColor Red
        Write-Host "💡 Make sure you're in the project directory" -ForegroundColor Yellow
    }
}

# Main script logic
try {
    # Check prerequisites
    if (-not (Test-WSL)) {
        exit 1
    }
    
    if (-not (Test-ProsodyInstalled)) {
        Write-Host "❌ Prosody is not installed in WSL" -ForegroundColor Red
        Write-Host "💡 Install with: wsl sudo apt update && wsl sudo apt install prosody" -ForegroundColor Yellow
        exit 1
    }
    
    # Execute requested action
    switch ($Action) {
        "start" {
            $prosodyStarted = Start-Prosody
            if ($prosodyStarted) {
                if ($ShowLogs) {
                    Write-Host "`n💡 Starting with logs visible. Press Ctrl+C to continue to app startup" -ForegroundColor Yellow
                    Start-Sleep -Seconds 2
                    Show-ProsodyLogs &
                    Start-Sleep -Seconds 3
                }
                
                Write-Host "`n🎯 Prosody is ready! Starting NTO development..." -ForegroundColor Green
                Start-Sleep -Seconds 2
                Start-Development
            } else {
                Write-Host "❌ Cannot start development - Prosody server failed to start" -ForegroundColor Red
                exit 1
            }
        }
        
        "stop" {
            Stop-Prosody | Out-Null
        }
        
        "restart" {
            Restart-Prosody | Out-Null
        }
        
        "status" {
            Show-ProsodyStatus
        }
        
        "logs" {
            Show-ProsodyLogs
        }
        
        "users" {
            Show-Users
        }
    }
}
catch {
    Write-Host "❌ Unexpected error: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

Write-Host "`n🏁 XMPP Development Manager finished" -ForegroundColor Green
