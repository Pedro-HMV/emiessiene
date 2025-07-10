# Test Build Script
# This script builds the application and runs basic validation

param(
    [switch]$Release,
    [switch]$SkipTests
)

Write-Host "🚀 Starting EmiEssiEne Build and Test Process" -ForegroundColor Green

# Set error action
$ErrorActionPreference = "Stop"

# Navigate to project root
$ProjectRoot = Split-Path -Path $PSScriptRoot -Parent | Split-Path -Parent
Set-Location $ProjectRoot

Write-Host "📍 Working directory: $ProjectRoot" -ForegroundColor Yellow

# Check if required tools are installed
Write-Host "🔍 Checking prerequisites..." -ForegroundColor Cyan

$RequiredTools = @(
    @{ Name = "cargo"; Command = "cargo --version" },
    @{ Name = "trunk"; Command = "trunk --version" }
)

foreach ($Tool in $RequiredTools) {
    try {
        $Version = Invoke-Expression $Tool.Command 2>$null
        Write-Host "✅ $($Tool.Name): $Version" -ForegroundColor Green
    }
    catch {
        Write-Host "❌ $($Tool.Name) not found. Please install it." -ForegroundColor Red
        exit 1
    }
}

# Build the project
Write-Host "🔨 Building the project..." -ForegroundColor Cyan

try {
    if ($Release) {
        Write-Host "Building in Release mode..." -ForegroundColor Yellow
        cargo tauri build --quiet
    } else {
        Write-Host "Building in Debug mode..." -ForegroundColor Yellow
        cargo build --quiet
        Set-Location "src-tauri"
        cargo build --quiet
        Set-Location ".."
    }
    Write-Host "✅ Build completed successfully!" -ForegroundColor Green
}
catch {
    Write-Host "❌ Build failed: $($_.Exception.Message)" -ForegroundColor Red
    exit 1
}

# Run tests if not skipped
if (-not $SkipTests) {
    Write-Host "🧪 Running tests..." -ForegroundColor Cyan
    
    try {
        # Run frontend tests
        Write-Host "Testing frontend..." -ForegroundColor Yellow
        cargo test --quiet
        
        # Run backend tests
        Write-Host "Testing backend..." -ForegroundColor Yellow
        Set-Location "src-tauri"
        cargo test --quiet
        Set-Location ".."
        
        Write-Host "✅ All tests passed!" -ForegroundColor Green
    }
    catch {
        Write-Host "❌ Tests failed: $($_.Exception.Message)" -ForegroundColor Red
        exit 1
    }
}

# Check for common issues
Write-Host "🔍 Running quality checks..." -ForegroundColor Cyan

# Check for TODO comments
$TodoCount = (Select-String -Path "src\**\*.rs" -Pattern "TODO|FIXME|XXX" -AllMatches).Count
if ($TodoCount -gt 0) {
    Write-Host "⚠️  Found $TodoCount TODO/FIXME comments" -ForegroundColor Yellow
}

# Check for warnings
Write-Host "Checking for warnings..." -ForegroundColor Yellow
$ClippyOutput = cargo clippy --quiet 2>&1
if ($ClippyOutput -match "warning") {
    Write-Host "⚠️  Clippy warnings found" -ForegroundColor Yellow
}

# Final summary
Write-Host "🎉 Build and test process completed!" -ForegroundColor Green
Write-Host "Summary:" -ForegroundColor Cyan
Write-Host "  - Build: ✅ Success" -ForegroundColor Green
if (-not $SkipTests) {
    Write-Host "  - Tests: ✅ Passed" -ForegroundColor Green
}
Write-Host "  - TODOs: $TodoCount" -ForegroundColor $(if ($TodoCount -gt 0) { "Yellow" } else { "Green" })

if ($Release) {
    Write-Host "📦 Release build available in: src-tauri\target\release\bundle\" -ForegroundColor Cyan
}

Write-Host "🏁 All done!" -ForegroundColor Green
