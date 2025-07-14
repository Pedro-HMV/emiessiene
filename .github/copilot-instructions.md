# GitHub Copilot Instructions for NTO Project

## Project Overview
This is a Rust-based desktop application that recreates the classic MSN Messenger experience using Tauri (backend) and Leptos (frontend). The project follows a specific structure and development practices.

## Development Platform
- **Primary Platform**: Windows
- **Shell**: PowerShell
- **Package Manager**: Cargo (Rust)
- **Build Tools**: Trunk (frontend), Tauri CLI (application)

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

### Frontend Dependencies
- Keep `Cargo.toml` updated with necessary dependencies
- Use specific versions for production builds
- Regularly update dependencies for security patches
- Document any special dependency requirements

### Backend Dependencies
- Maintain separate `Cargo.toml` for Tauri backend
- Use features flags to optimize bundle size
- Keep Tauri version updated for security and features
- Document any system-specific requirements

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

## Leptos 0.8 Migration Notes

### ✅ Migration Status: COMPLETED (July 10, 2025)
The project has been **successfully migrated** to Leptos 0.8.2.

### Key Changes Made
1. **Dependencies Updated**: Leptos 0.8.2, Leptos Router 0.8
2. **LocalResource API**: Fixed dereferencing patterns in app.rs  
3. **Build Status**: All compilation successful
4. **Development Server**: Running properly on http://localhost:1420

### Performance Optimizations Available
Use `--cfg=erase_components` for faster dev builds:
```toml
# In .cargo/config.toml
[build]
rustflags = ["--cfg=erase_components"]
```

Remember: This is a Windows-first project, so always consider Windows-specific requirements and use PowerShell for command-line operations!
