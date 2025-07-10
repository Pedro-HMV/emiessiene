# Development Guide

This guide provides detailed instructions for developing the EmiEssiEne project.

## 📋 Prerequisites

### Required Software

1. **Rust Toolchain**
   ```powershell
   # Install Rust via rustup
   # Download from https://rustup.rs/ or use:
   winget install Rustlang.Rustup
   
   # Verify installation
   rustc --version
   cargo --version
   ```

2. **Tauri Prerequisites**
   ```powershell
   # Install Microsoft C++ Build Tools
   # Download from: https://visualstudio.microsoft.com/visual-cpp-build-tools/
   
   # Install WebView2 (usually pre-installed on Windows 10/11)
   # Download from: https://developer.microsoft.com/microsoft-edge/webview2/
   ```

3. **Development Tools**
   ```powershell
   # Install Trunk for frontend building
   cargo install trunk
   
   # Install Tauri CLI
   cargo install tauri-cli
   
   # Optional: Install cargo-watch for file watching
   cargo install cargo-watch
   ```

### Recommended IDE Setup

- **Visual Studio Code** with extensions:
  - `rust-analyzer` - Rust language support
  - `Tauri` - Tauri-specific features
  - `leptos` - Leptos framework support (if available)

## 🏗️ Project Structure Deep Dive

### Frontend Structure (`src/`)

```
src/
├── app.rs                     # Main application entry and routing
├── main.rs                    # WASM entry point
├── components.rs              # Component module exports
└── components/
    ├── models.rs              # Shared data structures
    ├── loginpage_component.rs # Login interface
    ├── mainpage_component.rs  # Main application view
    ├── chat_component.rs      # Chat conversation UI
    ├── friend_component.rs    # Friend list items
    └── message_component.rs   # Message display
```

### Backend Structure (`src-tauri/`)

```
src-tauri/
├── src/
│   └── main.rs               # Tauri application and API commands
├── Cargo.toml                # Backend dependencies and metadata
├── tauri.conf.json           # Tauri configuration
├── build.rs                  # Build script
├── user.json                 # User data (development)
├── friends.json              # Friends data (development)
└── icons/                    # Application icons
```

### Configuration Files

- `Cargo.toml` - Frontend dependencies and workspace configuration
- `Trunk.toml` - Frontend build configuration
- `index.html` - HTML template
- `styles.css` - Application styling
- `leptosfmt.toml` - Code formatting for Leptos
- `rustfmt.toml` - Rust code formatting

## 💻 Development Workflow

### Starting Development

1. **Clone and Setup**
   ```powershell
   git clone <repository-url>
   cd emiessiene
   
   # Build dependencies
   cargo build
   cd src-tauri
   cargo build
   cd ..
   ```

2. **Development Server**
   ```powershell
   # Start full application with hot reload
   cargo tauri dev
   
   # This will:
   # - Build the Rust backend
   # - Start the Leptos frontend with Trunk
   # - Launch the Tauri window
   # - Enable hot reload for both frontend and backend
   ```

### Alternative Development Modes

#### Frontend Only Development
```powershell
# Useful for UI-only changes
trunk serve

# Access at http://localhost:8080
# Note: Backend functionality won't work
```

#### Backend Only Development
```powershell
cd src-tauri
cargo run
cd ..

# Tests backend logic without UI
```

#### Build Production Version
```powershell
# Create production build
cargo tauri build

# Output location: src-tauri/target/release/bundle/
```

## 🔧 Development Commands

### Useful Cargo Commands

```powershell
# Check code without building
cargo check

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy

# Clean build artifacts
cargo clean

# Watch for changes and rebuild
cargo watch -x check
```

### Tauri-Specific Commands

```powershell
# Development with debugging
cargo tauri dev --debug

# Build for specific target
cargo tauri build --target x86_64-pc-windows-msvc

# Generate Tauri icons
cargo tauri icon path/to/icon.png

# Info about Tauri environment
cargo tauri info
```

## 🏗️ Adding New Features

### Adding a New Component

1. **Create Component File**
   ```rust
   // src/components/new_component.rs
   use leptos::prelude::*;
   
   #[component]
   pub fn NewComponent() -> impl IntoView {
       view! {
           <div>"New Component"</div>
       }
   }
   ```

2. **Export in Module**
   ```rust
   // src/components.rs
   pub mod new_component;
   ```

3. **Use in Application**
   ```rust
   // src/app.rs
   use super::components::new_component::NewComponent;
   
   // Add to router or parent component
   ```

### Adding a New API Command

1. **Define in Backend**
   ```rust
   // src-tauri/src/main.rs
   #[command]
   fn new_command(state: App, param: String) -> Result<String, String> {
       // Implementation
       Ok("Success".to_string())
   }
   
   // Add to invoke_handler
   .invoke_handler(tauri::generate_handler![
       // existing commands...
       new_command,
   ])
   ```

2. **Call from Frontend**
   ```rust
   // In Leptos component
   use crate::app::invoke;
   use serde_wasm_bindgen::to_value;
   
   let call_command = move || {
       spawn_local(async move {
           let result = invoke("new_command", to_value(&"parameter").unwrap()).await;
           // Handle result
       });
   };
   ```

### Adding New Routes

```rust
// src/app.rs
<Routes fallback=move || view!{ "404 Not Found" }>
    <Route path=path!("/") view=LoginPage />
    <Route path=path!("/main") view=MainPage />
    <Route path=path!("/chat/:id") view=Chat />
    <Route path=path!("/new-route") view=NewComponent />
</Routes>
```

## 🎨 Styling Guidelines

### CSS Organization

- Use descriptive class names
- Follow BEM methodology where appropriate
- Group related styles together
- Use CSS custom properties for theming

### Component Styling

```rust
// Add classes to Leptos components
view! {
    <div class="component-name">
        <div class="component-name__element">
            // Content
        </div>
    </div>
}
```

### Adding New Styles

```css
/* styles.css */
.new-component {
    /* Base styles */
}

.new-component__element {
    /* Element styles */
}

.new-component--modifier {
    /* Modifier styles */
}
```

## 📊 State Management Patterns

### Global State with Context

```rust
// Provide context in app.rs
provide_context(signal_value);

// Consume in components
let context_value = use_context::<ReadSignal<T>>()
    .expect("Context not found");
```

### Local Component State

```rust
// In component
let (state, set_state) = signal(initial_value);

// Reactive updates
let derived = move || state.get() * 2;

// Effects for side effects
Effect::new(move |_| {
    // React to state changes
});
```

### Async Operations

```rust
// Using actions for async operations
let load_data = Action::new(move |_: &()| async move {
    // Async operation
    invoke("api_command", JsValue::null()).await
});

// Using resources for reactive async data
let data_resource = Resource::new(
    move || dependency.get(),
    move |_| async move {
        // Fetch data based on dependency
    }
);
```

## 🧪 Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_function() {
        // Test implementation
    }
}
```

### Integration Tests

```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_api_command() {
    // Test Tauri commands
}
```

### Frontend Testing

```rust
// Component testing with leptos_testing (when available)
#[test]
fn test_component() {
    // Test component behavior
}
```

## 🔍 Debugging

### Frontend Debugging

1. **Browser DevTools**
   - Open developer tools in Tauri window (F12)
   - Check console for JavaScript errors
   - Use debugger statements in generated code

2. **Leptos Debugging**
   ```rust
   // Add logging
   leptos::logging::log!("Debug message: {:?}", value);
   
   // Use web_sys console
   web_sys::console::log_1(&"Debug message".into());
   ```

### Backend Debugging

1. **Logging**
   ```rust
   use log::{info, debug, error};
   
   info!("Information message");
   debug!("Debug message");
   error!("Error message");
   ```

2. **Environment Variables**
   ```powershell
   # Enable debug logging
   $env:RUST_LOG="debug"
   cargo tauri dev
   ```

### Common Issues

1. **"Command not found" errors**
   - Check command is registered in `invoke_handler`
   - Verify command name spelling
   - Ensure proper parameter serialization

2. **Context not found errors**
   - Verify context is provided in parent component
   - Check context type matches expected type

3. **Build failures**
   - Run `cargo clean` and rebuild
   - Check for dependency conflicts
   - Verify Rust version compatibility

## 🚀 Performance Optimization

### Frontend Optimization

- Use `move ||` closures for reactive computations
- Minimize unnecessary re-renders with `untrack()`
- Optimize large lists with virtual scrolling
- Lazy load components when possible

### Backend Optimization

- Use async operations for I/O
- Implement proper error handling
- Cache frequently accessed data
- Optimize JSON parsing and serialization

## 📦 Deployment

### Building for Release

```powershell
# Create optimized build
cargo tauri build --release

# The output will be in:
# src-tauri/target/release/bundle/nsis/ (Windows installer)
# src-tauri/target/release/bundle/msi/ (MSI installer)
```

### Distribution

1. **Windows**
   - Use NSIS or MSI installer
   - Consider code signing for security
   - Test on different Windows versions

2. **Cross-platform**
   - Build on respective platforms
   - Use GitHub Actions for CI/CD
   - Maintain platform-specific configurations

## 🤝 Contribution Guidelines

1. **Code Style**
   - Run `cargo fmt` before committing
   - Address all `cargo clippy` warnings
   - Follow Rust naming conventions

2. **Documentation**
   - Update README for significant changes
   - Add inline documentation for public APIs
   - Include examples for complex features

3. **Testing**
   - Add tests for new functionality
   - Ensure existing tests pass
   - Test on target platforms

4. **Git Workflow**
   - Use descriptive commit messages
   - Create feature branches for new work
   - Keep commits focused and atomic
