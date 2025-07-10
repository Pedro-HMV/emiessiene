# Development Environment Setup Script
# This script sets up the development environment for EmiEssiEne

param(
    [switch]$Force
)

Write-Host "🔧 Setting up EmiEssiEne Development Environment" -ForegroundColor Green

$ErrorActionPreference = "Stop"

# Function to check if a command exists
function Test-Command {
    param([string]$Command)
    try {
        Get-Command $Command -ErrorAction Stop | Out-Null
        return $true
    }
    catch {
        return $false
    }
}

# Function to install Rust if not present
function Install-Rust {
    if (Test-Command "cargo") {
        Write-Host "✅ Rust is already installed" -ForegroundColor Green
        cargo --version
        return
    }
    
    Write-Host "📥 Installing Rust..." -ForegroundColor Yellow
    
    # Download and run rustup installer
    $RustupUrl = "https://win.rustup.rs/x86_64"
    $RustupPath = "$env:TEMP\rustup-init.exe"
    
    try {
        Invoke-WebRequest -Uri $RustupUrl -OutFile $RustupPath
        Start-Process -FilePath $RustupPath -ArgumentList "-y" -Wait
        
        # Refresh environment variables
        $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("PATH", "User")
        
        Write-Host "✅ Rust installed successfully!" -ForegroundColor Green
    }
    catch {
        Write-Host "❌ Failed to install Rust: $($_.Exception.Message)" -ForegroundColor Red
        exit 1
    }
    finally {
        if (Test-Path $RustupPath) {
            Remove-Item $RustupPath
        }
    }
}

# Function to install Trunk
function Install-Trunk {
    if (Test-Command "trunk") {
        Write-Host "✅ Trunk is already installed" -ForegroundColor Green
        trunk --version
        return
    }
    
    Write-Host "📥 Installing Trunk..." -ForegroundColor Yellow
    
    try {
        cargo install trunk
        Write-Host "✅ Trunk installed successfully!" -ForegroundColor Green
    }
    catch {
        Write-Host "❌ Failed to install Trunk: $($_.Exception.Message)" -ForegroundColor Red
        exit 1
    }
}

# Function to install Tauri CLI
function Install-TauriCLI {
    Write-Host "📥 Installing Tauri CLI..." -ForegroundColor Yellow
    
    try {
        cargo install tauri-cli
        Write-Host "✅ Tauri CLI installed successfully!" -ForegroundColor Green
    }
    catch {
        Write-Host "❌ Failed to install Tauri CLI: $($_.Exception.Message)" -ForegroundColor Red
        exit 1
    }
}

# Function to check Windows prerequisites
function Test-WindowsPrerequisites {
    Write-Host "🔍 Checking Windows prerequisites..." -ForegroundColor Cyan
    
    # Check for Visual Studio Build Tools
    $VSPath = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $VSPath) {
        $BuildTools = & $VSPath -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($BuildTools) {
            Write-Host "✅ Visual Studio Build Tools found" -ForegroundColor Green
        } else {
            Write-Host "⚠️  Visual Studio Build Tools not found. You may need to install them." -ForegroundColor Yellow
            Write-Host "   Download from: https://visualstudio.microsoft.com/visual-cpp-build-tools/" -ForegroundColor Cyan
        }
    } else {
        Write-Host "⚠️  Visual Studio Installer not found. Please install Visual Studio Build Tools." -ForegroundColor Yellow
    }
    
    # Check for WebView2
    $WebView2Paths = @(
        "${env:ProgramFiles(x86)}\Microsoft\EdgeWebView\Application",
        "${env:ProgramFiles}\Microsoft\EdgeWebView\Application"
    )
    
    $WebView2Found = $false
    foreach ($Path in $WebView2Paths) {
        if (Test-Path $Path) {
            $WebView2Found = $true
            break
        }
    }
    
    if ($WebView2Found) {
        Write-Host "✅ WebView2 runtime found" -ForegroundColor Green
    } else {
        Write-Host "⚠️  WebView2 runtime not found. It may be needed for the application." -ForegroundColor Yellow
        Write-Host "   Download from: https://developer.microsoft.com/microsoft-edge/webview2/" -ForegroundColor Cyan
    }
}

# Function to setup project dependencies
function Setup-ProjectDependencies {
    Write-Host "📦 Setting up project dependencies..." -ForegroundColor Cyan
    
    $ProjectRoot = Split-Path -Path $PSScriptRoot -Parent | Split-Path -Parent
    Set-Location $ProjectRoot
    
    Write-Host "Installing frontend dependencies..." -ForegroundColor Yellow
    cargo build
    
    Write-Host "Installing backend dependencies..." -ForegroundColor Yellow
    Set-Location "src-tauri"
    cargo build
    Set-Location ".."
    
    Write-Host "✅ Project dependencies installed!" -ForegroundColor Green
}

# Function to create development shortcuts
function Create-DevelopmentShortcuts {
    Write-Host "🔗 Creating development shortcuts..." -ForegroundColor Cyan
    
    $ProjectRoot = Split-Path -Path $PSScriptRoot -Parent | Split-Path -Parent
    $ShortcutsDir = Join-Path $ProjectRoot "shortcuts"
    
    if (-not (Test-Path $ShortcutsDir)) {
        New-Item -ItemType Directory -Path $ShortcutsDir | Out-Null
    }
    
    # Create development start script
    $DevScript = @"
# Start Development Server
Set-Location "$ProjectRoot"
cargo tauri dev
"@
    
    $DevScript | Out-File -FilePath (Join-Path $ShortcutsDir "start-dev.ps1") -Encoding UTF8
    
    # Create build script shortcut
    $BuildScript = @"
# Build Application
Set-Location "$ProjectRoot"
cargo tauri build
"@
    
    $BuildScript | Out-File -FilePath (Join-Path $ShortcutsDir "build.ps1") -Encoding UTF8
    
    Write-Host "✅ Development shortcuts created in $ShortcutsDir" -ForegroundColor Green
}

# Main setup process
Write-Host "🎯 Starting setup process..." -ForegroundColor Cyan

# Check Windows prerequisites
Test-WindowsPrerequisites

# Install Rust if needed
Install-Rust

# Install development tools
Install-Trunk
Install-TauriCLI

# Setup project
Setup-ProjectDependencies

# Create shortcuts
Create-DevelopmentShortcuts

# Final verification
Write-Host "🔍 Verifying installation..." -ForegroundColor Cyan

$Tools = @("cargo", "trunk", "cargo-tauri")
$AllGood = $true

foreach ($Tool in $Tools) {
    if (Test-Command $Tool) {
        Write-Host "✅ $Tool is available" -ForegroundColor Green
    } else {
        Write-Host "❌ $Tool is not available" -ForegroundColor Red
        $AllGood = $false
    }
}

if ($AllGood) {
    Write-Host "🎉 Development environment setup completed successfully!" -ForegroundColor Green
    Write-Host "Next steps:" -ForegroundColor Cyan
    Write-Host "  1. Run 'cargo tauri dev' to start development" -ForegroundColor White
    Write-Host "  2. Use 'cargo tauri build' to create a release build" -ForegroundColor White
    Write-Host "  3. Check out the documentation in the docs/ folder" -ForegroundColor White
} else {
    Write-Host "⚠️  Setup completed with some issues. Please review the errors above." -ForegroundColor Yellow
}

Write-Host "🏁 Setup script finished!" -ForegroundColor Green
