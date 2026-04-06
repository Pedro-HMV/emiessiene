# GitHub Copilot Instructions for NTO Project

## Project Overview
This is a Rust-based desktop application that recreates the classic MSN Messenger experience using Tauri (backend) and Leptos (frontend). The project follows a specific structure and development practices.

## ⚠️ Full-Stack Implementation Rule

**Every feature implementation MUST be complete end-to-end — backend AND frontend together.** Never implement one side without the other unless explicitly told to do only one side.

### Mandatory Checklist for Any New Feature

| Step | Backend | Frontend |
|------|---------|----------|
| 1 | Add `#[command] async fn my_cmd(...)` in `main.rs` | Add `#[derive(Serialize, Deserialize)] struct MyCmdArgs` in the component |
| 2 | Register command in `invoke_handler![]` | Call `invoke("my_cmd", to_value(&args).unwrap()).await` in a `spawn_local` block |
| 3 | If emitting events: `app_handle.emit_all("my_event", &payload)` | Set up a `listen("my_event", callback)` in a `spawn_local` at component mount |
| 4 | Run `cargo check -p nto` — zero errors required | Verify the component compiles and the UX flow is complete |

### Event Flow Pattern (Backend → Frontend)
```
Tauri command (async fn)  →  app_handle.emit_all("event_name", &payload)
                                        ↓
                          Frontend listen("event_name", callback)
                          updates Leptos signals → reactive UI
```

### Invoke Flow Pattern (Frontend → Backend)
```
Leptos spawn_local { invoke("cmd_name", args) }
         ↓
   #[command] async fn in main.rs
         ↓
   XmppManager / AppState method
         ↓
   Returns serde_json::Value { "success": bool, ... }
```

## Development Platform
- **Primary Platform**: Windows
- **Shell**: PowerShell
- **Package Manager**: Cargo (Rust)
- **Build Tools**: Trunk (frontend), Tauri CLI (application)

## Current Project State (April 2026)

### Routes
| Path | Component | File |
|------|-----------|------|
| `/` | `LoginPage` | `src/components/loginpage_component.rs` |
| `/register` | `RegisterPage` | `src/components/register_component.rs` |
| `/main` | `MainPage` | `src/components/mainpage_component.rs` |
| `/chat/:id` | `Chat` | `src/components/chat_component.rs` |

### Tauri Commands (registered in `invoke_handler![]`)
| Command | Description | Frontend caller |
|---------|-------------|----------------|
| `get_user` | Load user from `user.json` | `app.rs` LocalResource |
| `get_friends` | Load friends from `friends.json` | `app.rs` LocalResource |
| `update_friend` | Update friend data | `mainpage_component.rs` |
| `add_friend` | Add new friend | (available, not used in UI yet) |
| `update_username` | Change display name | `mainpage_component.rs` |
| `xmpp_connect` | Connect to XMPP server via direct-TLS | `loginpage_component.rs` |
| `xmpp_disconnect` | Disconnect from XMPP | (available) |
| `xmpp_send_message` | Send a chat message | `chat_component.rs` |
| `xmpp_set_presence` | Set availability/status | `loginpage_component.rs` (post-login) |
| `xmpp_add_contact` | Add a contact to roster | (available) |
| `xmpp_get_connection_status` | Poll connection state | `mainpage_component.rs` |
| `xmpp_register` | IBR registration (XEP-0077) | **NOT YET WIRED** — `register_component.rs` still uses a placeholder |

### Tauri Events (emitted by backend, listened to by frontend)
| Event | Payload type | Frontend listener |
|-------|-------------|-------------------|
| `xmpp_connected` | `ConnectionStatus { connected, jid, error }` | `mainpage_component.rs` |
| `xmpp_disconnected` | `ConnectionStatus` | `mainpage_component.rs` |
| `xmpp_message_received` | `XmppMessage { id, from, to, body, timestamp, message_type }` | `chat_component.rs` |
| `xmpp_presence_update` | `XmppPresence { jid, availability, status }` | **NOT YET WIRED** in frontend |
| `xmpp_roster_received` | `[{ jid, name, subscription }]` | **NOT YET WIRED** in frontend |

### Known Frontend Gaps (implement before shipping)
1. ~~`register_component.rs` — call `xmpp_register` instead of the `gloo_timers` placeholder~~ ✅ **DONE**
2. `mainpage_component.rs` — listen for `xmpp_roster_received` and populate friends list from XMPP
3. Any component — listen for `xmpp_presence_update` and update friend availability in real time

### XMPP Backend Architecture
- `src-tauri/src/xmpp_manager.rs` — `XmppManager` struct owns the connection
- Background `tokio::spawn(xmpp_event_loop(...))` drives the `tokio_xmpp::Client` stream
- `mpsc::Sender<OutgoingCmd>` channel for frontend commands → background loop
- Transport: direct-TLS (port 5223) via `tokio-xmpp 5` with `native-tls`
- IQ strategy: send via `client.send_stanza(Stanza::Iq(...))`, response arrives through the stream as `Event::Stanza(Stanza::Iq(...))`

## Project Structure Rules

### File Organization
- **Documentation**: All documentation goes in the `docs/` directory
- **Testing**: All test files, scripts, and utilities go in the `testing/` directory
- **Source Code**: 
  - Frontend code in `src/` directory
  - Backend code in `src-tauri/` directory
  - Components in `src/components/` directory
- **Configuration**: Keep configuration files in the project root only when appropriate
- **Assets**: Static assets go in `public/` directory

### Directory Structure
```
nto/
├── docs/                    # All documentation
├── testing/                 # Test scripts and utilities
├── src/                     # Frontend Leptos code
│   └── components/          # UI components
├── src-tauri/              # Backend Tauri code
├── public/                 # Static assets
└── [config files]          # Only appropriate config files in root
```

## Code Style and Practices

### Rust Code Standards
- Always run `cargo fmt` before committing
- Address all `cargo clippy` warnings
- Use meaningful variable and function names
- Follow Rust naming conventions (snake_case for functions, PascalCase for types)
- Add documentation comments for public APIs

### Leptos Component Guidelines
- Use `#[component]` macro for all components
- Keep components focused and reusable
- Use props for configuration and callbacks
- Implement proper error handling and loading states
- Use signals for reactive state management

### Tauri Backend Guidelines
- Use `#[command]` macro for API endpoints
- Return `Result<T, String>` for all commands
- Add proper logging with `log::info!()`, `log::error!()`, etc.
- Validate all input parameters
- Handle errors gracefully with meaningful messages

## Command Usage Rules

### Always Use PowerShell Commands
- **DO NOT** use Unix commands like `grep`, `tail`, `find`, `ls`, etc.
- **DO USE** PowerShell equivalents:
  - `Get-ChildItem` instead of `ls`
  - `Select-String` instead of `grep`
  - `Get-Content -Tail` instead of `tail`
  - `Get-Process` instead of `ps`
  - `Remove-Item` instead of `rm`

### Development Commands
```powershell
# Start development server
cargo tauri dev

# Build for production
cargo tauri build

# Frontend only development
trunk serve

# Check code without building
cargo check

# Format code
cargo fmt

# Lint code
cargo clippy

# Run tests
cargo test
```

## File Naming Conventions

### Component Files
- Use `snake_case` for component filenames
- Suffix with `_component.rs` for UI components
- Examples: `loginpage_component.rs`, `chat_component.rs`

### Documentation Files
- Use `UPPERCASE.md` for main documentation files
- Use `lowercase.md` for specific guides
- Examples: `README.md`, `API.md`, `development.md`

### Test Files
- Use `test_` prefix for test files
- Group related tests in modules
- Examples: `test_api.rs`, `test_components.rs`

## Development Workflow

### Adding New Features
1. Create feature branch with descriptive name
2. Add components in appropriate directories
3. Update documentation if needed
4. Add tests in `testing/` directory
5. Ensure all builds pass
6. Update relevant documentation

### Code Review Checklist
- [ ] Code follows Rust conventions
- [ ] Components are in correct directories
- [ ] Documentation is updated
- [ ] Tests are added for new functionality
- [ ] PowerShell commands are used (not Unix)
- [ ] Error handling is implemented
- [ ] Performance considerations are addressed

## Security Considerations

### Input Validation
- Always validate user inputs
- Sanitize data before processing
- Use type-safe interfaces between frontend and backend
- Handle errors gracefully without exposing internal details

### Data Storage
- Store user data securely
- Consider encryption for sensitive information
- Validate file paths to prevent directory traversal
- Use proper file permissions

## Performance Guidelines

### Frontend Performance
- Use `move ||` closures for reactive computations
- Minimize unnecessary re-renders
- Optimize large lists with virtual scrolling
- Lazy load components when possible

### Backend Performance
- Use async operations for I/O
- Cache frequently accessed data
- Optimize JSON parsing and serialization
- Use proper error handling to avoid panics

## Testing Strategy

### Test Organization
- Unit tests in respective modules
- Integration tests in `testing/` directory
- API tests for Tauri commands
- Component tests for UI functionality

### Test Categories
- **Unit Tests**: Individual function testing
- **Integration Tests**: Component interaction testing
- **API Tests**: Frontend-backend communication testing
- **End-to-End Tests**: Full user workflow testing

## Dependencies Management

### Frontend Dependencies (root `Cargo.toml`, package `nto-ui`)
- `leptos = "0.8.17"` with `csr` feature
- `leptos_router = "0.8.13"`
- `wasm-bindgen`, `wasm-bindgen-futures`, `js-sys`
- `serde`, `serde_json`, `serde-wasm-bindgen = "0.6.5"`
- `gloo-timers = "0.4.0"` with `futures` feature (async delays in WASM)
- `log`, `env_logger`, `console_error_panic_hook`

### Backend Dependencies (`src-tauri/Cargo.toml`, package `nto`)
- `tauri = "1"` with `shell-open` feature
- `tokio = "1.0"` with `full` feature
- `tokio-xmpp = "5"` — `default-features = false`, features `["direct-tls", "native-tls"]`
- `jid = "0.12"`, `minidom = "0.18"`, `native-tls = "0.2"`, `tokio-native-tls = "0.3"`
- `uuid = "1.0"` with `v4` feature, `chrono = "0.4"` with `serde` feature
- `serde`, `serde_json`, `futures = "0.3"`, `log`, `env_logger`
- Keep Tauri version updated for security and features
- Use feature flags to optimize bundle size

## Documentation Standards

### Code Documentation
- Use `///` for public API documentation
- Include examples for complex functions
- Document error conditions and return values
- Keep documentation up-to-date with code changes

### Project Documentation
- Update README for significant changes
- Maintain API documentation in `docs/API.md`
- Keep development guide current in `docs/DEVELOPMENT.md`
- Document deployment procedures

## Common Pitfalls to Avoid

### Platform-Specific Issues
- Don't assume Unix tools are available
- Use PowerShell for all command-line operations
- Test on Windows-specific environments
- Handle Windows path separators correctly

### Rust-Specific Issues
- Don't ignore `cargo clippy` warnings
- Handle `Result` types properly (don't unwrap blindly)
- Use appropriate lifetime annotations
- Avoid unnecessary cloning

### Leptos-Specific Issues
- Don't forget to provide required contexts
- Use signals appropriately for reactivity
- Handle async operations with proper error handling
- Avoid prop drilling by using context when appropriate

### Tauri-Specific Issues
- Register all commands in `invoke_handler`
- Use proper serialization/deserialization
- Handle file system operations securely
- Test both development and production builds

## IDE and Editor Configuration

### Recommended Extensions
- `rust-analyzer` for Rust language support
- `Tauri` for Tauri-specific features
- `leptos` for Leptos framework support (if available)
- `PowerShell` for script editing

### Settings
- Enable format on save
- Configure clippy integration
- Set up proper indentation (4 spaces for Rust)
- Enable error highlighting

## Deployment Considerations

### Build Process
- Use `cargo tauri build` for production builds
- Test builds on target platforms
- Consider code signing for Windows
- Generate proper installer packages

### Distribution
- Use appropriate installer formats (MSI, NSIS for Windows)
- Include all necessary dependencies
- Test installation process
- Document system requirements

## Version Control

### Git Workflow
- Use descriptive commit messages
- Keep commits focused and atomic
- Create feature branches for new work
- Regularly merge from main branch

### Branch Naming
- Use descriptive branch names
- Include issue numbers when relevant
- Examples: `feature/chat-history`, `bugfix/login-validation`

## Troubleshooting

### Common Issues
- Build errors: Run `cargo clean` and rebuild
- Missing dependencies: Check `Cargo.toml` files
- Runtime errors: Check browser console and terminal output
- Performance issues: Profile with appropriate tools

### Debug Strategies
- Use `console.log()` for frontend debugging
- Use `log::debug!()` for backend debugging
- Enable debug mode with `RUST_LOG=debug`
- Use browser developer tools for UI debugging

## Leptos 0.8 Notes

### ✅ Migration Status: COMPLETED — Currently on Leptos 0.8.17 / Router 0.8.13

### Key Patterns in Use
1. `LocalResource::new(|| async { invoke(...) })` for loading data from Tauri on mount
2. `Effect::new(move |_| { ... })` to react to resource changes and update signals
3. `provide_context` / `use_context` for global state (user, friends, open_chats)
4. `spawn_local(async move { ... })` for all async Tauri invocations inside event handlers
5. `wasm_bindgen::closure::Closure::wrap(Box::new(...) as Box<dyn Fn(JsValue)>)` for Tauri event listeners — always call `.forget()` to keep callbacks alive
6. `from_value::<T>(js_value)` / `to_value(&struct)` via `serde_wasm_bindgen` for all data crossing WASM boundary

### Pattern for Tauri Event Listener (WASM)
```rust
spawn_local(async move {
    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
        async fn listen(event: &str, callback: &js_sys::Function) -> JsValue;
    }
    let cb = Closure::wrap(Box::new(move |event: JsValue| {
        // parse and update signals here
    }) as Box<dyn Fn(JsValue)>);
    let _ = listen("event_name", cb.as_ref().unchecked_ref()).await;
    cb.forget(); // REQUIRED — or the callback is dropped immediately
});
```

### Performance Optimizations Available
Use `--cfg=erase_components` for faster dev builds:
```toml
# In .cargo/config.toml
[build]
rustflags = ["--cfg=erase_components"]
```

Remember: This is a Windows-first project, so always consider Windows-specific requirements and use PowerShell for command-line operations!
