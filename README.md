# EmiEssiEne - MSN Messenger Clone

A modern recreation of the classic MSN Messenger instant messaging client, built with Rust, Tauri, and Leptos.

[![Built with Tauri](https://img.shields.io/badge/Tauri-24C8D8?style=for-the-badge&logo=tauri&logoColor=white)](https://tauri.app/)
[![Built with Leptos](https://img.shields.io/badge/Leptos-EF3939?style=for-the-badge&logo=rust&logoColor=white)](https://leptos.dev/)
[![Built with Rust](https://img.shields.io/badge/Rust-CE422B?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)

## 🎯 Project Overview

**EmiEssiEne** is a desktop application that recreates the nostalgic MSN Messenger experience with modern technology. The name is a playful Portuguese reference to "MSN" that captures the spirit of the original application.

### ✨ Features

- 🔐 **User Authentication** - Classic login interface with status selection
- 👥 **Friend Management** - Online/offline friend lists with status messages
- 💬 **Real-time Messaging** - Individual chat conversations
- 📱 **Tabbed Interface** - Multiple simultaneous chat windows
- 🎨 **Classic Design** - Faithful recreation of the original MSN Messenger UI
- ⚡ **Modern Performance** - Built with Rust for speed and reliability

## 🚀 Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)
- [Trunk](https://trunkrs.dev/) - `cargo install trunk`
- [Tauri CLI](https://tauri.app/) - `cargo install tauri-cli`

### Development

```powershell
# Clone the repository
git clone [repository-url]
cd emiessiene

# Start development server
cargo tauri dev
```

### Building

```powershell
# Build for production
cargo tauri build
```

## 📚 Documentation

- **[Complete Documentation](./docs/README.md)** - Comprehensive project overview
- **[Development Guide](./docs/DEVELOPMENT.md)** - Detailed development instructions
- **[API Reference](./docs/API.md)** - Frontend-backend communication
- **[Testing Guide](./testing/README.md)** - Testing procedures and scripts

## 🏗️ Architecture

### Technology Stack

- **Frontend**: Leptos (Rust web framework)
- **Backend**: Tauri (Rust desktop framework)
- **Build Tools**: Trunk (WASM bundler)
- **Styling**: Custom CSS (MSN Messenger inspired)

### Project Structure

```
emiessiene/
├── docs/                    # 📚 Documentation
├── testing/                 # 🧪 Test scripts and utilities
├── src/                     # 🎨 Frontend (Leptos)
│   └── components/          # UI components
├── src-tauri/              # ⚙️ Backend (Tauri)
├── public/                 # 📁 Static assets
└── [config files]          # ⚙️ Configuration
```

## 🎮 Usage

1. **Login**: Start with the classic MSN login screen
2. **Main Interface**: View your friends list and manage your profile
3. **Chat**: Click on friends to start conversations
4. **Multiple Chats**: Open multiple chat windows simultaneously

## 🔧 Development

### Current Status
🚧 **Migration in Progress**: The project is currently being upgraded from Leptos 0.7.8 to 0.8.x for improved performance and new features.

### Getting Started

1. **Setup Environment**: Run `.\testing\scripts\setup-dev.ps1` (Windows PowerShell)
2. **Start Development**: `cargo tauri dev`
3. **Build Project**: `cargo tauri build`

### Key Commands

```powershell
# Development server with hot reload
cargo tauri dev

# Build for production
cargo tauri build

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

## 🧪 Testing

### Automated Testing

```powershell
# Run all tests
cargo test

# Run build and test script
.\testing\scripts\build-test.ps1
```

### Manual Testing

See [manual testing checklist](./testing/manual-testing.md) for comprehensive testing procedures.

## 🤝 Contributing

1. **Follow the Project Structure** - Keep files in appropriate directories
2. **Use PowerShell** - This is a Windows-first project
3. **Follow Rust Conventions** - Use `cargo fmt` and `cargo clippy`
4. **Update Documentation** - Keep docs up to date
5. **Test Your Changes** - Run tests before submitting

See [Copilot Instructions](./.github/copilot-instructions.md) for detailed development guidelines.

## 🔒 Security

- Built with Rust for memory safety
- Tauri provides secure native APIs
- Local data storage with validation
- Type-safe frontend-backend communication

## 🐛 Troubleshooting

### Common Issues

1. **Build Errors**: Run `cargo clean` and rebuild
2. **Missing Dependencies**: Check [Development Guide](./docs/DEVELOPMENT.md#prerequisites)
3. **Runtime Issues**: Check browser console and terminal output

### Getting Help

- Review the [documentation](./docs/)
- Check the [testing guide](./testing/)
- Look at existing [issues](../../issues) (if using Git)

## 📄 License

[Add your license here]

## 🙏 Acknowledgments

- Original MSN Messenger for inspiration
- [Leptos](https://leptos.dev/) and [Tauri](https://tauri.app/) communities
- All contributors and testers

## 📸 Screenshots

[Add screenshots here when available]

---

**EmiEssiEne** - Bringing back the golden age of instant messaging! 🌟
