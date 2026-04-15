# Development Guide

This guide provides detailed instructions for developing, packaging, and deploying the NTO project.

The project is a Tauri v1 + Leptos 0.8 WASM desktop app. The backend handles XMPP connectivity (via `tokio-xmpp`), persists login preferences to the OS app-data folder, and exposes Tauri commands to the frontend. The frontend is a reactive WASM app rendered inside a WebView2 window.

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
├── app.rs                     # Main application entry, routing, and global signal context
├── main.rs                    # WASM entry point
├── components.rs              # Component module exports
└── components/
    ├── models.rs              # Shared data structures (User, Friend, Availability)
    ├── loginpage_component.rs # Login interface (XMPP connect, remember me, auto sign-in)
    ├── register_component.rs  # Account registration (XEP-0077 in-band registration)
    ├── mainpage_component.rs  # Friends list, XMPP listeners, sign out, vCard, presence
    ├── chat_component.rs      # Chat window — message history, input, avatars
    ├── friend_component.rs    # Friend list item component
    └── message_component.rs  # Message bubble display
```

### Backend Structure (`src-tauri/`)

```
src-tauri/
├── src/
│   ├── main.rs               # Tauri commands, AppState, XMPP bridge
│   └── xmpp_manager.rs       # XMPP connection, event loop, roster, vCard, presence
├── Cargo.toml                # Backend dependencies and metadata
├── tauri.conf.json           # Tauri configuration (min window size, identifier)
├── build.rs                  # Build script
└── icons/                    # Application icons
```

> Note: `user.json` and `friends.json` are no longer used. Friends are populated at runtime
> from the XMPP roster. Login credentials are stored in `%APPDATA%\nto\nto_remembered.json`.

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
   cd nto
   
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
# Create production build (Windows installer)
cargo tauri build

# Output:
#   src-tauri/target/release/bundle/nsis/*.exe  (NSIS installer)
#   src-tauri/target/release/bundle/msi/*.msi   (MSI installer)
```

No code changes are needed between dev and release builds — `#[cfg(not(debug_assertions))]`
switches automatically. Before shipping, verify `tauri.conf.json` has the correct
`identifier`, `productName`, and `version`.

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

4. **XMPP data missing after login (e.g. empty friends list, no vCard)**
   - This is almost always an event timing / race condition
   - The backend emits events the instant they arrive from the server
   - If the frontend component hasn't mounted yet, the event fires into the void — **Tauri does not buffer or replay events**
   - **Fix pattern**: always call a re-fetch command on component mount *after* registering listeners:
     ```rust
     // In the mount spawn_local, AFTER setting up all listen() callbacks:
     let _ = invoke("xmpp_request_roster", to_value(&serde_json::json!({})).unwrap()).await;
     let _ = invoke("xmpp_fetch_vcard",    to_value(&serde_json::json!({})).unwrap()).await;
     // drain_pending_subscriptions for buffered pre-mount arrivals
     let result = invoke("get_pending_subscriptions", ...).await;
     ```
   - For events that may arrive repeatedly before mount, buffer them in `XmppManager` (see `pending_subscriptions`) and expose a drain command

5. **Duplicate pending friend requests (contact shows in Pending after already being accepted)**
   - Caused by ejabberd echoing the mutual `subscribe` presence back as a new incoming `subscribe`
   - Guard the `xmpp_subscription_request` listener: skip adding a JID to pending if it is already present in either the online or offline friends list
   - The `xmpp_roster_push` listener should also call `set_pending_requests.update(|r| r.retain(...))` when a contact moves to `"both"/"from"/"to"` subscription state

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

## 📦 Packaging & Distribution

### Building the Windows Installer

```powershell
cargo tauri build
```

Output artifacts (ready to distribute):
- `src-tauri/target/release/bundle/nsis/*.exe` — NSIS installer
- `src-tauri/target/release/bundle/msi/*.msi` — MSI installer

No code changes are needed between dev and release — `#[cfg(not(debug_assertions))]` handles it automatically.

**Before releasing**, verify in `src-tauri/tauri.conf.json`:
- `identifier` — reverse-domain app identifier
- `productName` — shown to users in Windows installer
- `version` — bumped appropriately

### Testing the Release Build Locally

```powershell
# Build without bundling (faster iteration)
cargo build --release -p nto

# Run the release binary directly
.\target\release\emiessiene.exe
```

---

## 🌐 XMPP Server Setup (for Testing & Production)

NTO requires a real XMPP server. For personal/small-scale use, Oracle Cloud Always Free provides a permanently free ARM VM large enough to run Prosody indefinitely.

### Oracle Cloud Free Tier VM (Recommended)

**Spec**: VM.Standard.A1.Flex — 4 OCPUs, 24 GB RAM, 200 GB disk, 10 TB/month outbound — **free forever**.

#### One-time VM provisioning

1. Create an Oracle Cloud account at [cloud.oracle.com](https://cloud.oracle.com) and choose a home region.
2. Compute → Instances → Create Instance → Shape: **VM.Standard.A1.Flex** → 4 OCPUs, 24 GB RAM.
3. OS: **Ubuntu 22.04 LTS** (Always Free eligible). Generate and download the SSH key pair.
4. In the VCN Security List, add Ingress rules:
   - TCP 22 (SSH)
   - TCP 5222 (XMPP STARTTLS)
   - TCP 5223 (XMPP direct-TLS)

#### Install and configure Prosody

```bash
sudo apt update && sudo apt install prosody -y
```

Minimal `/etc/prosody/prosody.cfg.lua`:

```lua
VirtualHost "your-domain.com"
  ssl = {
    key = "/etc/prosody/certs/your-domain.com.key",
    certificate = "/etc/prosody/certs/your-domain.com.crt"
  }
  -- For LAN/dev testing with a raw IP, use the IP as the domain name

modules_enabled = {
  "saslauth", "roster", "vcard", "register",
  "ping", "dialback", "carbons", "private",
}

allow_registration = true  -- disable once accounts are created
```

Generate a self-signed cert (for testing — replace with Let's Encrypt for production):

```bash
sudo prosodyctl cert generate your-domain.com
sudo systemctl restart prosody
```

Create accounts:

```bash
sudo prosodyctl register alice your-domain.com password123
sudo prosodyctl register bob   your-domain.com password456
```

NTO login JID format: `alice@your-domain.com`

#### Keepalive cron (mandatory — prevents Oracle reclaiming idle VMs)

Oracle reclaims VMs if CPU/network/memory stays below 20% for 7 consecutive days.

```bash
# /etc/cron.d/oci-keepalive
0 */6 * * * root dd if=/dev/urandom of=/tmp/kv bs=1M count=50 2>/dev/null && rm /tmp/kv
```

#### Backup Prosody data to OCI Object Storage

OCI gives 20 GB of Object Storage that **survives VM deletion** (it's separate infrastructure). Back up `/var/lib/prosody/` daily.

```bash
#!/bin/bash
# /usr/local/bin/backup-prosody.sh
DATE=$(date +%Y%m%d)
tar czf /tmp/prosody-$DATE.tar.gz /var/lib/prosody/
oci os object put \
  --bucket-name prosody-backups \
  --file /tmp/prosody-$DATE.tar.gz \
  --name prosody-$DATE.tar.gz
rm /tmp/prosody-$DATE.tar.gz
```

```bash
# /etc/cron.d/prosody-backup
0 3 * * * root /usr/local/bin/backup-prosody.sh
```

#### Recovery after VM loss

```bash
# 1. Spin up a new free ARM VM and install Prosody
# 2. Download latest backup:
oci os object get --bucket-name prosody-backups --name prosody-YYYYMMDD.tar.gz --file /tmp/r.tar.gz
# 3. Restore:
sudo tar xzf /tmp/r.tar.gz -C /
sudo chown -R prosody:prosody /var/lib/prosody
sudo systemctl restart prosody
```

Maximum data loss: 24 hours (with daily backups).

### Cost Comparison

| Option | Cost | Notes |
|---|---|---|
| Oracle Cloud ARM A1 | **$0/month** | Always Free; keepalive cron required |
| Linode Nanode 1 GB | $5/month | No idle-reclamation risk; zero maintenance |
| Linode 2 GB | $12/month | Comfortable headroom for OS + Prosody |

For a personal project, Oracle Free is the obvious choice. For reliability without any maintenance overhead, the $5 Linode Nanode is sufficient.

---

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
