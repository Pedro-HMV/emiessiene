# EmiEssiEne - MSN Messenger Clone

A modern recreation of the classic MSN Messenger instant messaging client, built using Rust, Tauri, and Leptos. This project recreates the nostalgic experience of MSN Messenger with a contemporary tech stack.

## 🎯 Project Overview

**EmiEssiEne** is a desktop application that mimics the interface and functionality of the classic MSN Messenger. The name is a playful Portuguese reference to "MSN" that captures the nostalgic spirit of the original application.

### Core Features

- **User Authentication**: Login interface with status selection (Online, Away, Busy, Offline)
- **Friend Management**: Display of online and offline friends with status messages
- **Real-time Messaging**: Chat interface with individual friend conversations
- **Status Management**: Users can set custom status messages and availability
- **Tabbed Conversations**: Support for multiple simultaneous chats
- **Classic UI Design**: Faithful recreation of the original MSN Messenger interface

## 🏗️ Architecture

### Technology Stack

- **Frontend Framework**: [Leptos](https://leptos.dev/) - A full-stack, isomorphic Rust web framework
- **Desktop Framework**: [Tauri](https://tauri.app/) - Build smaller, faster, and more secure desktop applications
- **Language**: Rust (both frontend and backend)
- **Build Tool**: [Trunk](https://trunkrs.dev/) - WASM web application bundler for Rust
- **Styling**: CSS with custom styles mimicking MSN Messenger

### Project Structure

```
emiessiene/
├── src/                           # Frontend Leptos application
│   ├── app.rs                     # Main app router and context providers
│   ├── main.rs                    # Application entry point
│   ├── components.rs              # Module declarations
│   └── components/                # UI components
│       ├── models.rs              # Shared data models
│       ├── loginpage_component.rs # Login screen
│       ├── mainpage_component.rs  # Main application interface
│       ├── chat_component.rs      # Chat conversation UI
│       ├── friend_component.rs    # Friend list item
│       └── message_component.rs   # Chat message display
├── src-tauri/                     # Backend Tauri application
│   ├── src/main.rs               # Tauri backend with API commands
│   ├── Cargo.toml                # Backend dependencies
│   ├── tauri.conf.json           # Tauri configuration
│   ├── user.json                 # User data storage
│   └── friends.json              # Friends list storage
├── public/                        # Static assets
├── docs/                          # Documentation
├── testing/                       # Test scripts and utilities
├── index.html                     # HTML template
├── styles.css                     # Main stylesheet
├── Cargo.toml                     # Frontend dependencies
└── Trunk.toml                     # Trunk configuration
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
   cd emiessiene
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

### User Journey

1. **Login Screen** (`/`)
   - User enters credentials
   - Selects availability status
   - Options for "Remember me" and "Auto sign in"

2. **Main Interface** (`/main`)
   - User profile display with editable username
   - Friends list (online/offline sections)
   - Status message display
   - Friend search functionality

3. **Chat Interface** (`/chat/:id`)
   - Individual conversation window
   - Message input with send functionality
   - Chat controls (voice, video, files, etc.)
   - User avatars and status indicators

### Data Flow

1. **Frontend (Leptos)**
   - Handles UI rendering and user interactions
   - Manages reactive state using signals
   - Communicates with backend via Tauri commands

2. **Backend (Tauri)**
   - Provides secure native APIs
   - Manages user and friend data storage
   - Handles application state management

3. **Data Storage**
   - `user.json`: Current user information
   - `friends.json`: Friends list with status data

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

- **User**: Name, email, status message, availability
- **Friend**: Name, email, status message, availability
- **Availability**: Enum (Online, Away, Busy, Offline)
- **UpdateUsernameArgs**: API parameter structure

## 🔧 API Reference

### Tauri Commands

The backend exposes the following commands for frontend communication:

#### User Management
- `get_user()`: Retrieve current user information
- `update_username(name: String)`: Update user's display name

#### Friend Management
- `get_friends()`: Get sorted friends list (online, offline)
- `add_friend(name, email, status?, availability?)`: Add new friend
- `update_friend(email, name?, status?, availability?)`: Update friend information

### Frontend State Management

#### Context Providers
- `user: ReadSignal<User>`: Current user state
- `friends: ReadSignal<(Vec<Friend>, Vec<Friend>)>`: Online and offline friends
- `open_chats: ReadSignal<Vec<usize>>`: List of active chat tabs

#### Local State
- Component-specific signals for UI state
- Actions for async operations
- Effects for reactive updates

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
- **Data Storage**: Local JSON files for development (consider encryption for production)
- **API Commands**: Validated inputs and error handling
- **Frontend Validation**: Input sanitization and type safety

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
   - Verify JSON data file integrity

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
