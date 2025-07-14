# Troubleshooting Guide

This guide helps resolve common issues when developing or running the NTO application.

## 🔧 Build Issues

### "cargo: command not found"

**Symptoms**: PowerShell can't find the `cargo` command
**Solution**: 
1. Install Rust from [rustup.rs](https://rustup.rs/)
2. Restart PowerShell
3. Verify with `cargo --version`

### "trunk: command not found"

**Symptoms**: Can't find `trunk` command when building frontend
**Solution**: 
```powershell
cargo install trunk
```

### "Failed to resolve dependencies"

**Symptoms**: Cargo can't download dependencies
**Solution**: 
1. Check internet connection
2. Update cargo index: `cargo update`
3. Clear cargo cache: `Remove-Item -Recurse -Force ~/.cargo/registry/cache/`

### "Build failed with exit code 101"

**Symptoms**: Build process terminates with errors
**Solution**: 
1. Clean the build: `cargo clean`
2. Rebuild: `cargo build`
3. Check for syntax errors in recent changes

## 🖥️ Runtime Issues

### "Application won't start"

**Symptoms**: Application fails to launch or crashes immediately
**Solution**: 
1. Check if WebView2 is installed
2. Run in development mode: `cargo tauri dev`
3. Check console output for error messages

### "White screen on startup"

**Symptoms**: Application window opens but shows blank/white screen
**Solution**: 
1. Open DevTools (F12) and check console
2. Verify frontend build completed successfully
3. Check if `index.html` and `styles.css` are accessible

### "Backend API not responding"

**Symptoms**: Frontend can't communicate with backend
**Solution**: 
1. Verify Tauri commands are registered in `invoke_handler`
2. Check command names match exactly
3. Verify parameter serialization

## 🎨 UI/Styling Issues

### "Styles not loading"

**Symptoms**: Application appears unstyled or has broken layout
**Solution**: 
1. Check if `styles.css` exists and is linked in `index.html`
2. Verify Trunk build process includes CSS
3. Check for CSS syntax errors

### "Responsive design broken"

**Symptoms**: Layout doesn't adapt to window size changes
**Solution**: 
1. Check CSS media queries
2. Verify flexbox/grid properties
3. Test with different window sizes

## 🔄 State Management Issues

### "Context not found" errors

**Symptoms**: Components can't access provided contexts
**Solution**: 
1. Verify context is provided in parent component
2. Check context type matches exactly
3. Ensure component is within provider scope

### "Signals not updating"

**Symptoms**: UI doesn't react to state changes
**Solution**: 
1. Check if signal is being read in reactive context
2. Verify signal updates are called correctly
3. Use `logging::log!()` to debug signal values

## 📡 Data Issues

### "Failed to load user data"

**Symptoms**: Application can't load user information
**Solution**: 
1. Check if `src-tauri/user.json` exists
2. Verify JSON format is valid
3. Check file permissions

### "Friends list not loading"

**Symptoms**: Friends list appears empty or doesn't load
**Solution**: 
1. Check if `src-tauri/friends.json` exists
2. Verify JSON array format
3. Check for malformed friend objects

### "Data not persisting"

**Symptoms**: Changes don't save between sessions
**Solution**: 
1. Check if backend commands are saving to files
2. Verify file write permissions
3. Check if data is being serialized correctly

## 🧪 Development Issues

### "Hot reload not working"

**Symptoms**: Changes don't appear without manual refresh
**Solution**: 
1. Restart development server: `cargo tauri dev`
2. Check if files are being watched correctly
3. Verify file changes are saved

### "IDE not providing Rust support"

**Symptoms**: No syntax highlighting or code completion
**Solution**: 
1. Install `rust-analyzer` extension
2. Verify Rust toolchain is installed
3. Open project as workspace folder

## 💻 Platform-Specific Issues

### Windows-Specific

#### "MSVC build tools not found"
**Solution**: Install Visual Studio Build Tools from Microsoft

#### "WebView2 runtime missing"
**Solution**: 
1. Install WebView2 runtime from Microsoft
2. Or install Microsoft Edge (includes WebView2)

#### "Access denied" errors
**Solution**: 
1. Run PowerShell as Administrator
2. Check file permissions
3. Verify antivirus isn't blocking files

## 🔍 Debugging Techniques

### Frontend Debugging

1. **Browser DevTools**: Press F12 in the application window
2. **Console Logging**: Use `web_sys::console::log_1(&"message".into())`
3. **Leptos Debugging**: Use `leptos::logging::log!("message")`

### Backend Debugging

1. **Logging**: Use `log::info!()`, `log::debug!()`, `log::error!()`
2. **Environment Variables**: Set `RUST_LOG=debug` for detailed logging
3. **Print Debugging**: Use `println!()` for quick debugging

### Network Debugging

1. **Check API Calls**: Use DevTools Network tab
2. **Verify Command Names**: Ensure exact spelling matches
3. **Parameter Validation**: Check serialization/deserialization

## 🚨 Error Messages

### Common Error Messages and Solutions

#### "thread 'main' panicked at ..."
**Cause**: Rust panic in backend code
**Solution**: 
1. Check the panic message for details
2. Add proper error handling
3. Use `Result` types instead of `unwrap()`

#### "TypeError: Cannot read property ... of undefined"
**Cause**: JavaScript error in frontend
**Solution**: 
1. Check if data structure matches expectations
2. Add null checks
3. Verify async operations complete

#### "Failed to invoke command"
**Cause**: Tauri command invocation failed
**Solution**: 
1. Check command is registered
2. Verify parameter types
3. Check return type matches

## 📋 Diagnostic Commands

### System Information
```powershell
# Check Rust installation
cargo --version
rustc --version

# Check Node.js (if needed)
node --version
npm --version

# Check system info
Get-ComputerInfo | Select-Object WindowsProductName, WindowsVersion
```

### Build Information
```powershell
# Check build targets
rustup target list --installed

# Check dependencies
cargo tree

# Check for outdated packages
cargo outdated
```

### Performance Monitoring
```powershell
# Check memory usage
Get-Process | Where-Object {$_.ProcessName -like "*nto*"}

# Monitor file changes
Get-ChildItem -Path . -Recurse | Where-Object {$_.LastWriteTime -gt (Get-Date).AddMinutes(-5)}
```

## 🆘 Getting Help

### Before Asking for Help

1. **Check the Documentation**: Review all docs in the `docs/` folder
2. **Search Existing Issues**: Look for similar problems
3. **Try Clean Build**: Run `cargo clean` and rebuild
4. **Check Recent Changes**: What changed since it last worked?

### When Reporting Issues

Include this information:
- **System Information**: OS version, Rust version, etc.
- **Steps to Reproduce**: Exact steps that cause the issue
- **Expected Behavior**: What should happen
- **Actual Behavior**: What actually happens
- **Error Messages**: Full error text and stack traces
- **Screenshots**: If UI-related issues

### Useful Resources

- [Tauri Documentation](https://tauri.app/v1/guides/)
- [Leptos Documentation](https://leptos.dev/)
- [Rust Documentation](https://doc.rust-lang.org/)
- [WebAssembly Documentation](https://webassembly.org/)

## 🔧 Recovery Procedures

### Complete Reset
If all else fails:
```powershell
# Clean everything
cargo clean
Remove-Item -Recurse -Force target/
Remove-Item -Recurse -Force src-tauri/target/

# Rebuild from scratch
cargo build
cd src-tauri
cargo build
cd ..
```

### Dependency Reset
```powershell
# Update all dependencies
cargo update

# Or reset to specific versions
cargo install cargo-edit
cargo set-version 0.1.0
```

### Configuration Reset
1. Backup any custom configuration
2. Reset to default configuration files
3. Gradually re-apply customizations

---

If you encounter an issue not covered here, please add it to help others!
