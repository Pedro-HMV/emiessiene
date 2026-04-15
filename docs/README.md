# NTO - MSN Messenger Clone

A modern recreation of the classic MSN Messenger instant messaging client, built using Rust, Tauri, and Leptos. This project recreates the nostalgic experience of MSN Messenger with a contemporary tech stack.

## 🎯 Project Overview

**NTO** is a desktop application that mimics the interface and functionality of the classic MSN Messenger. The name is a playful reference to "MSN" that captures the nostalgic spirit of the original.

The app connects to a real XMPP server using direct-TLS (port 5223), making it a functional instant messenger — not just a mock UI.

### Core Features

- **User Authentication**: Login with XMPP credentials (JID + password). Remember me and auto sign-in support.
- **Account Registration**: Create new accounts directly from the app via in-band registration (XEP-0077).
- **Friend Management**: Live XMPP roster with online/offline status display and real-time updates.
- **vCard Display Names**: Contacts show their vCard nickname, not a raw JID local-part.
- **Real-time Messaging**: Chat interface with individual conversations and message history.
- **Presence & Status**: Set availability (Online, Away, Busy) with a custom mood message.
- **Tabbed Conversations**: Multiple simultaneous chat tabs.
- **Classic MSN UI**: Faithful recreation of the original MSN Messenger interface.

## 🏗️ Architecture

### Technology Stack

- **Frontend Framework**: [Leptos](https://leptos.dev/) - A full-stack, isomorphic Rust web framework
- **Desktop Framework**: [Tauri](https://tauri.app/) - Build smaller, faster, and more secure desktop applications
- **Language**: Rust (both frontend and backend)
- **Build Tool**: [Trunk](https://trunkrs.dev/) - WASM web application bundler for Rust
- **Styling**: CSS with custom styles mimicking MSN Messenger

### Project Structure

```
nto/
├── src/                           # Frontend Leptos application (compiles to WASM)
│   ├── app.rs                     # Router, global signals, context providers, sign-out
│   ├── main.rs                    # WASM entry point
│   ├── components.rs              # Module declarations
│   └── components/                # UI components
│       ├── models.rs              # Shared data models (User, Friend, Availability)
│       ├── loginpage_component.rs # Login screen — XMPP connect, remember me
│       ├── register_component.rs  # Registration screen (XEP-0077)
│       ├── mainpage_component.rs  # Main interface — friends, XMPP listeners, vCard
│       ├── chat_component.rs      # Chat window — messages, input, avatars
│       ├── friend_component.rs    # Friend list item
│       └── message_component.rs  # Message bubble
├── src-tauri/                     # Backend Tauri application (native Rust)
│   ├── src/
│   │   ├── main.rs               # Tauri commands, AppState, XMPP bridge
│   │   └── xmpp_manager.rs       # XMPP connection, event loop, roster, vCard, presence
│   ├── Cargo.toml                # Backend dependencies
│   ├── tauri.conf.json           # Tauri configuration
│   └── icons/                    # App icons
├── public/                        # Static assets
├── docs/                          # Documentation
├── testing/                       # Test scripts and utilities
├── index.html                     # HTML template
├── styles.css                     # Main stylesheet
├── Cargo.toml                     # Frontend dependencies
└── Trunk.toml                     # Trunk build configuration
```

## 🚀 Getting Started

### Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs/)
- **Trunk**: Install with `cargo install trunk`
- **Tauri CLI**: Install with `cargo install tauri-cli`
- **Node.js**: For any additional tooling (optional)

### Development Setup

1. **Clone the Repository**
   ```powershell
   git clone [repository-url]
   cd nto
   ```

2. **Install Dependencies**
   ```powershell
   # Install Rust dependencies for frontend
   cargo build
   
   # Install Rust dependencies for backend
   cd src-tauri
   cargo build
   cd ..
   ```

3. **Development Workflow**

   **Option A: Full Development Mode (Recommended)**
   ```powershell
   # Run both frontend and backend in development mode
   cargo tauri dev
   ```

   **Option B: Frontend Only**
   ```powershell
   # Serve frontend only (for UI development)
   trunk serve
   ```

   **Option C: Backend Only**
   ```powershell
   # Test backend functionality
   cd src-tauri
   cargo run
   cd ..
   ```

### Building for Production

```powershell
# Build the complete application
cargo tauri build
```

The built application will be available in `src-tauri/target/release/bundle/`.

## 📱 Application Flow

### Routes

| Path | Component | Purpose |
|------|-----------|--------|
| `/` | `LoginPage` | XMPP login with remember-me, auto sign-in, availability picker |
| `/register` | `RegisterPage` | New account creation via XEP-0077 |
| `/main` | `MainPage` | Friends list, presence, vCard, open chat tabs, sign out |
| `/chat/:id` | `Chat` | Individual conversation window with message history |

### Data Flow

1. **Frontend (Leptos WASM)** — renders UI reactively using Leptos signals; talks to backend via `invoke()` (request/response) and `listen()` (event push).

2. **Backend (Tauri native Rust)** — manages the XMPP connection in a background tokio task, emits events to the frontend via `emit_all()`, and persists login preferences to `%APPDATA%\nto\nto_remembered.json`.

3. **XMPP Server** — authoritative source for roster, presence, messages. NTO uses direct-TLS on port 5223.

4. **State reset on sign-out** — `App` holds all global signals (`user`, `friends`, `open_chats`, `messages`). The sign-out callback in `app.rs` resets all four signals before navigating back to `/`.

## 🎨 Component Architecture

### Core Components

#### `App` (app.rs)
- Main application router
- Context providers for global state
- Resource management for user and friends data

#### `LoginPage` (loginpage_component.rs)
- User authentication interface
- Status selection dropdown
- Navigation to main application

#### `MainPage` (mainpage_component.rs)
- Primary application interface
- User profile management
- Friends list display
- Chat tab management

#### `Chat` (chat_component.rs)
- Individual conversation interface
- Message display and input
- Chat controls and navigation

#### `Friend` (friend_component.rs)
- Friend list item component
- Status indicator and information display
- Click handlers for opening chats

#### `Message` (message_component.rs)
- Individual message display
- User attribution and content rendering

### Data Models

Located in `src/components/models.rs`:

- **User**: `name`, `email`, `flavour_text`, `availability`
- **Friend**: `name`, `email`, `flavour_text`, `availability`
- **Availability**: Enum (`Online`, `Away`, `Busy`, `Offline`)

## 🔧 API Reference

See [API.md](API.md) for the full command and event reference. Key commands:

| Command | Description |
|---------|-------------|
| `xmpp_connect` | Connect to XMPP server (direct-TLS port 5223) |
| `xmpp_register` | Create account via XEP-0077 |
| `xmpp_request_roster` | Fetch contacts from server |
| `xmpp_send_message` | Send a chat message |
| `xmpp_set_presence` | Broadcast availability + mood |
| `xmpp_fetch_vcard` | Fetch own vCard (name + flavour text) |
| `xmpp_add_contact` | Add contact + subscribe |
| `xmpp_accept_subscription` | Accept inbound friend request |

Key events pushed from backend to frontend:

| Event | Trigger |
|-------|---------|
| `xmpp_roster_received` | Full roster loaded |
| `xmpp_presence_update` | Contact came online / went offline |
| `xmpp_message_received` | Incoming chat message |
| `xmpp_subscription_request` | Someone wants to add you |
| `xmpp_vcard_received` | vCard data ready |

## 🎭 Styling and Theming

The application uses custom CSS to recreate the classic MSN Messenger look:

- **Color Scheme**: Blue gradients and classic Windows XP styling
- **Typography**: Sans-serif fonts with various weights
- **Layout**: Flexbox-based responsive design
- **Icons**: Unicode emoji characters for compatibility

Key style classes:
- `.flex-col`, `.flex-row`: Layout utilities
- `.main_bordered`: Classic border styling
- `.chat_*`: Chat-specific component styles
- `.friend_container`: Friend list item styling

## 🔒 Security Considerations

- **Tauri Security**: Application runs with restricted permissions
- **Credentials**: Stored in `%APPDATA%\nto\nto_remembered.json` — never hardcoded
- **TLS**: All XMPP traffic is encrypted via direct-TLS (port 5223)
- **API Commands**: Validated inputs and error handling
- **Frontend Validation**: Input sanitization and type safety

## 🚀 Deployment

### Building for Windows distribution

```powershell
cargo tauri build
# Output: src-tauri/target/release/bundle/nsis/*.exe
#         src-tauri/target/release/bundle/msi/*.msi
```

### XMPP Server (Oracle Cloud Free Tier)

The recommended hosting option is an Oracle Cloud Always Free ARM instance (VM.Standard.A1.Flex): 4 OCPUs, 24 GB RAM, 200 GB storage, 10 TB/month outbound — permanently free.

Key requirements:
- Install **Prosody** XMPP server on Ubuntu 22.04
- Open ports 5222 (STARTTLS) and 5223 (direct-TLS) in the VCN Security List
- Add a keepalive cron job to prevent Oracle reclaiming idle VMs
- Back up `/var/lib/prosody/` to OCI Object Storage (20 GB free) daily

See [DEVELOPMENT.md](DEVELOPMENT.md#-xmpp-server-setup-for-testing--production) for full step-by-step instructions.

## 🧪 Development Guidelines

### Code Style
- Follow Rust conventions (rustfmt, clippy)
- Use meaningful component and variable names
- Implement proper error handling
- Add documentation for public APIs

### State Management
- Use Leptos signals for reactive state
- Provide context for shared data
- Minimize prop drilling with context providers
- Handle async operations with resources and actions

### Component Design
- Keep components focused and reusable
- Use props for configuration and callbacks
- Implement proper lifecycle management
- Handle loading and error states

## 🐛 Troubleshooting

### Common Issues

1. **Build Errors**
   - Ensure Rust and Tauri are up to date
   - Check for missing dependencies
   - Verify Trunk installation

2. **Runtime Errors**
   - Check browser console for frontend errors
   - Review Tauri logs for backend issues
   - XMPP connection errors appear in the login page error field

3. **Empty friends list after login**
   - This is an event-timing issue — the roster event fired before the MainPage listener was registered
   - The app re-fetches the roster on mount via `xmpp_request_roster` to handle this automatically

3. **Styling Issues**
   - Ensure CSS is properly linked
   - Check for class name conflicts
   - Verify responsive design breakpoints

### Development Tips

- Use `cargo tauri dev` for live reload during development
- Monitor the browser console for frontend debugging
- Check the terminal output for backend logging
- Use Rust analyzer for better IDE support

## 🤝 Contributing

1. Follow the established project structure
2. Write tests for new functionality
3. Update documentation for significant changes
4. Follow Rust and web development best practices
5. Test on multiple platforms before submitting

## 📝 License

[Add license information]

## 🙏 Acknowledgments

- Original MSN Messenger for inspiration
- Leptos and Tauri communities for excellent documentation
- Contributors and testers

---

For more detailed information, see individual component documentation in the respective source files.
