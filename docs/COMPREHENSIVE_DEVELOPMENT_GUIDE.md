# EmiEssiene - Comprehensive Development Guide

## Table of Contents
1. [Project Overview](#project-overview)
2. [Architecture & Technology Stack](#architecture--technology-stack)
3. [Development Environment Setup](#development-environment-setup)
4. [Project Structure](#project-structure)
5. [Key Components & Modules](#key-components--modules)
6. [State Management](#state-management)
7. [Routing & Navigation](#routing--navigation)
8. [Backend API](#backend-api)
9. [Build & Deployment](#build--deployment)
10. [Development Workflow](#development-workflow)
11. [Testing Strategy](#testing-strategy)
12. [Common Issues & Solutions](#common-issues--solutions)
13. [Contributing Guidelines](#contributing-guidelines)
14. [Leptos 0.8 Migration Notes](#leptos-08-migration-notes)

## Project Overview

EmiEssiene is a desktop application that recreates the classic MSN Messenger experience, built with modern web technologies. The project uses Tauri for the desktop application framework and Leptos for the reactive web UI framework.

### Key Features
- **Classic MSN Messenger UI** - Authentic recreation of the classic MSN Messenger interface
- **User Authentication** - Login system with user profiles
- **Friends Management** - Add, remove, and manage friends list
- **Real-time Chat** - Instant messaging with multiple chat windows
- **Status Management** - Online/offline status with custom status messages
- **Desktop Integration** - Native desktop application with system tray support

### Target Audience
- Developers wanting to contribute to the project
- Anyone interested in understanding the codebase
- Users wanting to customize or extend the application

## Architecture & Technology Stack

### Frontend Stack
- **Leptos 0.7.8** - Reactive web framework (scheduled for 0.8 upgrade)
- **Rust** - Primary programming language
- **WebAssembly (WASM)** - Compilation target for web components
- **CSS** - Styling with custom CSS resembling MSN Messenger
- **Trunk** - Build tool for Rust + WASM projects

### Backend Stack
- **Tauri 1.x** - Desktop application framework
- **Rust** - Backend API and system integration
- **JSON** - Data storage and API communication
- **File System** - Local storage for user data and friends

### Development Tools
- **Cargo** - Rust package manager and build system
- **PowerShell** - Primary shell for Windows development
- **VS Code** - Recommended IDE with Rust and Tauri extensions
- **Git** - Version control

## Development Environment Setup

### Prerequisites
- **Windows 10/11** (Primary development platform)
- **Rust** (latest stable version)
- **Node.js** (for Trunk and development tools)
- **PowerShell** (default shell)
- **Git** (for version control)

### Installation Steps

1. **Install Rust**
   ```powershell
   # Install Rust via rustup
   Invoke-WebRequest -Uri https://win.rustup.rs/ -OutFile rustup-init.exe
   .\rustup-init.exe
   ```

2. **Install Tauri CLI**
   ```powershell
   cargo install tauri-cli
   ```

3. **Install Trunk**
   ```powershell
   cargo install trunk
   ```

4. **Install WASM target**
   ```powershell
   rustup target add wasm32-unknown-unknown
   ```

5. **Clone Repository**
   ```powershell
   git clone https://github.com/Pedro-HMV/emiessiene.git
   cd emiessiene
   ```

### IDE Setup
- Install **rust-analyzer** extension for VS Code
- Install **Tauri** extension for VS Code
- Install **PowerShell** extension for VS Code
- Configure format-on-save with `cargo fmt`

## Project Structure

```
emiessiene/
├── docs/                           # All documentation
│   ├── API.md                     # API documentation
│   ├── DEVELOPMENT.md             # Development guide
│   ├── TROUBLESHOOTING.md         # Common issues and solutions
│   └── COMPREHENSIVE_DEVELOPMENT_GUIDE.md  # This file
├── testing/                       # Test scripts and utilities
│   ├── scripts/                   # PowerShell test scripts
│   └── manual-testing.md          # Manual testing procedures
├── src/                          # Frontend Leptos code
│   ├── main.rs                   # Main entry point
│   ├── app.rs                    # App configuration and Tauri bindings
│   ├── components.rs             # Component module exports
│   └── components/               # UI components
│       ├── loginpage_component.rs    # Login page component
│       ├── mainpage_component.rs     # Main messenger interface
│       ├── chat_component.rs         # Chat window component
│       ├── friend_component.rs       # Friend list item component
│       ├── message_component.rs      # Message bubble component
│       └── models.rs                 # Data models and types
├── src-tauri/                    # Backend Tauri code
│   ├── src/
│   │   └── main.rs              # Tauri backend entry point
│   ├── Cargo.toml               # Backend dependencies
│   ├── tauri.conf.json          # Tauri configuration
│   ├── friends.json             # Sample friends data
│   └── user.json                # Sample user data
├── public/                       # Static assets
├── target/                       # Build artifacts (auto-generated)
├── Cargo.toml                    # Frontend dependencies
├── Trunk.toml                    # Trunk configuration
├── index.html                    # Main HTML template
└── styles.css                    # Global styles
```

### File Organization Rules
- **Documentation**: All documentation goes in `docs/`
- **Testing**: All test files, scripts, and utilities go in `testing/`
- **Source Code**: Frontend in `src/`, Backend in `src-tauri/`
- **Components**: UI components in `src/components/`
- **Configuration**: Keep config files in project root only when appropriate
- **Assets**: Static assets go in `public/`

## Key Components & Modules

### Frontend Components

#### LoginPage Component (`src/components/loginpage_component.rs`)
- **Purpose**: User authentication interface
- **Features**: Username/password input, login validation
- **State**: Form inputs, loading state, error handling
- **Navigation**: Redirects to main page on successful login

#### MainPage Component (`src/components/mainpage_component.rs`)
- **Purpose**: Main messenger interface
- **Features**: User profile display, friends list, chat tabs
- **State**: User info, friends list, open chats
- **Context**: Provides friends and chat context to child components

#### Chat Component (`src/components/chat_component.rs`)
- **Purpose**: Individual chat window
- **Features**: Message display, message input, chat history
- **State**: Messages, input text, chat partner info
- **Real-time**: Handles message sending/receiving

#### Friend Component (`src/components/friend_component.rs`)
- **Purpose**: Friend list item display
- **Features**: Friend name, status, avatar, availability indicator
- **State**: Friend information, online status
- **Interactions**: Click to open chat, status display

#### Message Component (`src/components/message_component.rs`)
- **Purpose**: Individual message bubble
- **Features**: Message text, timestamp, sender info
- **State**: Message content, metadata
- **Styling**: Different styles for sent/received messages

### Data Models (`src/components/models.rs`)

#### User Model
```rust
pub struct User {
    pub name: String,
    pub email: String,
    pub status: String,
    pub availability: Availability,
}
```

#### Friend Model
```rust
pub struct Friend {
    pub name: String,
    pub status: String,
    pub availability: Availability,
}
```

#### Availability Enum
```rust
pub enum Availability {
    Online,
    Away,
    Busy,
    Offline,
}
```

## State Management

### Leptos Signals
The application uses Leptos's signal-based reactivity system for state management.

#### Global State
- **User State**: Current user information (name, email, status, availability)
- **Friends State**: List of friends divided into online/offline
- **Chat State**: Open chat windows and their content

#### Context Providers
- **Friends Context**: Provides friends list to all components
- **Chat Context**: Manages open chat windows and their state

#### State Flow
1. **Login**: User state is initialized from backend
2. **Friends**: Friends list is loaded from backend and provided via context
3. **Chat**: Chat windows are managed through routing and context
4. **Real-time Updates**: State updates trigger reactive UI updates

### Data Persistence
- **User Data**: Stored in `src-tauri/user.json`
- **Friends Data**: Stored in `src-tauri/friends.json`
- **Chat History**: Currently in-memory (future: persistent storage)

## Routing & Navigation

### Leptos Router
The application uses Leptos Router for client-side navigation.

#### Route Structure
```rust
<Router>
    <Routes>
        <Route path="/" view=LoginPage />
        <Route path="/main" view=MainPage />
        <Route path="/chat/:id" view=ChatComponent />
    </Routes>
</Router>
```

#### Navigation Patterns
- **Login to Main**: Automatic redirect after successful authentication
- **Main to Chat**: Click on friend opens chat window
- **Chat Tabs**: Multiple chat windows managed through routing
- **Back Navigation**: Proper cleanup of chat state

### URL Structure
- `/` - Login page
- `/main` - Main messenger interface
- `/chat/:id` - Individual chat window (id = friend index)

## Backend API

### Tauri Commands
Backend functionality is exposed through Tauri commands that can be called from the frontend.

#### Available Commands
- `update_username` - Updates user's display name
- `get_friends` - Retrieves friends list
- `send_message` - Sends a message (future implementation)
- `get_messages` - Retrieves chat history (future implementation)

#### Command Implementation
```rust
#[tauri::command]
async fn update_username(name: &str) -> Result<User, String> {
    // Implementation here
}
```

### Data Storage
- **Format**: JSON files for simple data storage
- **Location**: `src-tauri/` directory
- **Structure**: Separate files for users and friends
- **Future**: Consider database integration for production

## Build & Deployment

### Development Build
```powershell
# Start development server with hot reload
cargo tauri dev

# Frontend only (for UI development)
trunk serve
```

### Production Build
```powershell
# Build for production
cargo tauri build

# This creates:
# - Executable in target/release/
# - Installer packages in target/release/bundle/
```

### Build Configuration
- **Trunk.toml**: Frontend build configuration
- **tauri.conf.json**: Tauri application configuration
- **Cargo.toml**: Dependencies and build settings

### Deployment Options
- **Standalone Executable**: Direct distribution of .exe file
- **Installer Package**: MSI or NSIS installer for Windows
- **Portable Version**: Self-contained application bundle

## Development Workflow

### Daily Development
1. **Start Development Server**
   ```powershell
   cargo tauri dev
   ```

2. **Make Changes**
   - Edit source files in `src/` or `src-tauri/`
   - Hot reload automatically updates the application

3. **Testing**
   - Manual testing during development
   - Run test scripts from `testing/scripts/`

4. **Code Quality**
   ```powershell
   # Format code
   cargo fmt
   
   # Check for issues
   cargo clippy
   
   # Run tests
   cargo test
   ```

### Feature Development
1. **Create Feature Branch**
   ```powershell
   git checkout -b feature/new-feature
   ```

2. **Implement Feature**
   - Add components to appropriate directories
   - Update documentation if needed
   - Add tests for new functionality

3. **Test Feature**
   - Manual testing
   - Automated tests if applicable
   - Cross-platform testing

4. **Code Review**
   - Ensure code follows project conventions
   - Check for PowerShell command usage (not Unix commands)
   - Validate error handling

### Common Development Tasks

#### Adding a New Component
1. Create component file in `src/components/`
2. Export component in `src/components.rs`
3. Update routing if needed
4. Add styles to `styles.css`

#### Adding a New Tauri Command
1. Add command function to `src-tauri/src/main.rs`
2. Register command in `invoke_handler`
3. Add frontend bindings in `src/app.rs`
4. Use command in components

#### Updating Dependencies
```powershell
# Update Cargo.toml
# Then run:
cargo update
```

## Testing Strategy

### Test Categories
- **Unit Tests**: Individual component and function testing
- **Integration Tests**: Component interaction testing
- **Manual Tests**: User workflow testing
- **Build Tests**: Ensure clean builds across environments

### Test Organization
- **Unit Tests**: In respective component files with `#[cfg(test)]`
- **Integration Tests**: In `testing/` directory
- **Manual Tests**: Documented procedures in `testing/manual-testing.md`
- **Build Tests**: PowerShell scripts in `testing/scripts/`

### Testing Commands
```powershell
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run integration tests
cargo test --test integration_tests
```

## Common Issues & Solutions

### Build Issues
- **Problem**: Trunk build fails
  - **Solution**: Ensure WASM target is installed: `rustup target add wasm32-unknown-unknown`

- **Problem**: Tauri build fails
  - **Solution**: Check Tauri prerequisites: https://tauri.app/v1/guides/getting-started/prerequisites

### Runtime Issues
- **Problem**: Hot reload not working
  - **Solution**: Restart development server, check file permissions

- **Problem**: Context not found errors
  - **Solution**: Ensure context providers are properly set up in component hierarchy

### Platform-Specific Issues
- **Problem**: Unix commands not working
  - **Solution**: Use PowerShell equivalents (Get-ChildItem vs ls, Select-String vs grep)

## Contributing Guidelines

### Code Style
- Follow Rust naming conventions (snake_case, PascalCase)
- Use `cargo fmt` before committing
- Address `cargo clippy` warnings
- Add documentation for public APIs

### Commit Guidelines
- Use descriptive commit messages
- Keep commits focused and atomic
- Reference issues when applicable

### Pull Request Process
1. Create feature branch
2. Implement changes with tests
3. Update documentation
4. Ensure all checks pass
5. Request review

### Code Review Checklist
- [ ] Code follows Rust conventions
- [ ] Components are in correct directories
- [ ] Documentation is updated
- [ ] Tests are added for new functionality
- [ ] PowerShell commands are used (not Unix)
- [ ] Error handling is implemented
- [ ] Performance considerations are addressed

## Leptos 0.8 Migration Notes

### Current Status
The project is currently using Leptos 0.7.8 and needs to be upgraded to 0.8.x.

### Key Breaking Changes in 0.8
1. **LocalResource API**: Remove `.as_deref()` calls when using `LocalResource`
2. **Server Function Errors**: Custom error types must implement `FromServerFnError`
3. **Axum 0.8**: Updated to Axum 0.8 (breaking change for re-exported types)
4. **Removed Defaults**: `LeptosOptions` and `ConfFile` no longer have `Default` impl
5. **Signal API**: `SignalSetter` now in prelude

### Migration Steps
1. **Update Dependencies**
   ```toml
   leptos = "0.8"
   leptos_router = "0.8"
   ```

2. **Fix LocalResource Usage**
   - Remove `.as_deref()` calls
   - Update to new API structure

3. **Update Error Handling**
   - Implement `FromServerFnError` for custom errors
   - Update server function error handling

4. **Test Thoroughly**
   - Ensure all components still work
   - Check routing functionality
   - Verify state management

### Benefits of 0.8 Upgrade
- **Performance**: Better compile times with `--cfg=erase_components`
- **WebSocket Support**: New server function WebSocket capabilities
- **Better Error Handling**: More ergonomic error handling
- **Islands Router**: Improved routing for complex applications

---

## Conclusion

This guide provides a comprehensive overview of the EmiEssiene project for developers. The project aims to recreate the classic MSN Messenger experience using modern web technologies, with a focus on maintainability, performance, and user experience.

For specific implementation details, refer to the source code and individual component documentation. For issues and questions, check the troubleshooting guide or create an issue in the repository.

**Remember**: This is a Windows-first project, so always use PowerShell commands and consider Windows-specific requirements when contributing.
