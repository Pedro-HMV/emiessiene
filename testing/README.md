# Testing Guide

This directory contains test scripts, utilities, and documentation for testing the EmiEssiEne application.

## Test Categories

### Unit Tests
Located within individual modules using `#[cfg(test)]` blocks.

### Integration Tests
Located in this directory for testing component interactions.

### API Tests
Tests for Tauri command functionality and frontend-backend communication.

### Manual Testing Scripts
PowerShell scripts for manual testing scenarios.

## Running Tests

### All Tests
```powershell
# Run all tests in the project
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name
```

### Frontend Tests
```powershell
# Run frontend-specific tests
cd src
cargo test
```

### Backend Tests
```powershell
# Run backend-specific tests
cd src-tauri
cargo test
```

## Test Data

Test data files are located in the `test-data/` subdirectory.

## Test Scripts

PowerShell scripts for automated testing are located in the `scripts/` subdirectory.

## Manual Testing

See `manual-testing.md` for manual testing procedures and checklists.
