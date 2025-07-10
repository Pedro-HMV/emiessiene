# Manual Testing Checklist

This document provides a comprehensive checklist for manually testing the EmiEssiEne application.

## Pre-Testing Setup

- [ ] Ensure the application builds successfully: `cargo tauri build`
- [ ] Verify development server starts: `cargo tauri dev`
- [ ] Check that all dependencies are installed
- [ ] Verify test data files exist (`user.json`, `friends.json`)

## Login Page Testing

### Basic Functionality
- [ ] Application opens to login page
- [ ] Username field accepts input
- [ ] Password field accepts input and masks characters
- [ ] Status dropdown displays all options (Online, Away, Busy, Offline)
- [ ] "Remember me" checkbox functions
- [ ] "Sign me in automatically" checkbox functions
- [ ] "Sign In" button navigates to main page

### UI/UX Testing
- [ ] Page layout matches MSN Messenger style
- [ ] Avatar placeholder displays correctly
- [ ] Form elements are properly aligned
- [ ] Responsive design works on different window sizes
- [ ] All text is readable and properly styled

### Error Handling
- [ ] Empty fields handling (if validation exists)
- [ ] Invalid credentials handling (if validation exists)
- [ ] Network error handling (if applicable)

## Main Page Testing

### User Profile Section
- [ ] User avatar displays correctly
- [ ] Username displays and is editable (click to edit)
- [ ] Status message displays correctly
- [ ] Availability status displays with correct icon
- [ ] Username update saves correctly
- [ ] Edit mode can be cancelled (ESC key)
- [ ] Enter key saves username changes
- [ ] "Sign Out" link returns to login page

### Friends List
- [ ] Friends list loads and displays correctly
- [ ] Online friends appear in "Friends" section
- [ ] Offline friends appear in "Offline" section
- [ ] Friends are sorted alphabetically within sections
- [ ] Friend status icons display correctly
- [ ] Friend status messages display correctly
- [ ] Friend names are clickable and open chats
- [ ] Empty friend sections display appropriately

### Navigation
- [ ] "Find a friend" input field displays
- [ ] Friend search functionality (if implemented)
- [ ] Add friend functionality (if implemented)
- [ ] Page layout is responsive

## Chat Interface Testing

### Chat Opening
- [ ] Clicking friend name opens chat window
- [ ] Chat window displays friend information correctly
- [ ] Multiple chats can be opened simultaneously
- [ ] Chat tabs display at the top of main interface
- [ ] Active chat tab is highlighted
- [ ] Chat navigation works correctly

### Chat Functionality
- [ ] Message input field accepts text
- [ ] Send button sends messages
- [ ] Enter key sends messages
- [ ] Shift+Enter creates new line (if supported)
- [ ] Messages display in chat window
- [ ] Message attribution shows correct sender
- [ ] Timestamps display correctly (if implemented)
- [ ] Chat history persists during session

### Chat Controls
- [ ] Back button returns to main interface
- [ ] Close button (❌) closes chat
- [ ] Settings button (⚙️) displays correctly
- [ ] Voice button (📞) displays correctly
- [ ] Video button (📷) displays correctly
- [ ] File button (📁) displays correctly
- [ ] All toolbar buttons are properly styled

### Chat UI
- [ ] Friend avatar displays in chat
- [ ] Friend status displays correctly
- [ ] Chat layout matches MSN Messenger style
- [ ] Message bubbles display correctly
- [ ] Scroll functionality works for long conversations
- [ ] Window resizing works properly

## Performance Testing

### Application Startup
- [ ] Application starts within reasonable time (< 5 seconds)
- [ ] Initial data loads quickly
- [ ] No memory leaks during startup
- [ ] UI is responsive during loading

### Runtime Performance
- [ ] Typing in input fields is responsive
- [ ] Friend list scrolling is smooth
- [ ] Chat window scrolling is smooth
- [ ] No significant memory growth during usage
- [ ] CPU usage remains reasonable

### Data Operations
- [ ] Username updates happen quickly
- [ ] Friend list updates in real-time
- [ ] Chat messages send without delay
- [ ] File operations (if any) are responsive

## Cross-Platform Testing (if applicable)

### Windows Specific
- [ ] Application integrates with Windows properly
- [ ] Window controls (minimize, maximize, close) work
- [ ] Application icon displays correctly
- [ ] Installer works properly (if available)
- [ ] Uninstaller works properly (if available)

### Different Screen Sizes
- [ ] Application works on 1920x1080 resolution
- [ ] Application works on 1366x768 resolution
- [ ] Application works on high-DPI displays
- [ ] Minimum window size is usable
- [ ] Maximum window size displays correctly

## Error Handling Testing

### Network Issues
- [ ] Application handles offline mode gracefully
- [ ] Error messages are user-friendly
- [ ] Application recovers from network issues
- [ ] Data persistence works during network problems

### Data Issues
- [ ] Missing user.json file handling
- [ ] Missing friends.json file handling
- [ ] Corrupted data file handling
- [ ] Large friends list handling

### Edge Cases
- [ ] Very long usernames
- [ ] Very long status messages
- [ ] Special characters in text fields
- [ ] Unicode/emoji support
- [ ] Empty strings handling

## Accessibility Testing

### Keyboard Navigation
- [ ] Tab order is logical
- [ ] All interactive elements are keyboard accessible
- [ ] Enter and Space keys work for buttons
- [ ] Escape key cancels operations appropriately

### Visual Accessibility
- [ ] Text is readable with sufficient contrast
- [ ] Focus indicators are visible
- [ ] Important information isn't conveyed by color alone
- [ ] Text can be resized without breaking layout

## Security Testing

### Input Validation
- [ ] No script injection possible in text fields
- [ ] File path validation works correctly
- [ ] Data sanitization prevents crashes
- [ ] Error messages don't expose sensitive information

### Data Privacy
- [ ] User data is stored securely
- [ ] No sensitive information in logs
- [ ] Data files have appropriate permissions
- [ ] Application doesn't transmit unexpected data

## Regression Testing

### Core Features
- [ ] All previously working features still work
- [ ] No new crashes introduced
- [ ] Performance hasn't degraded
- [ ] UI consistency maintained

### New Features
- [ ] New features don't break existing functionality
- [ ] New features work as designed
- [ ] New features have proper error handling
- [ ] New features follow established patterns

## Test Environment Notes

### System Requirements
- Operating System: Windows 10/11
- RAM: Minimum 4GB recommended
- Storage: 100MB free space
- Network: Optional (for future features)

### Test Data
- Use provided test user and friends data
- Test with various data sizes
- Test with edge case data (empty, very long, special characters)

### Reporting Issues
When reporting issues, include:
- Steps to reproduce
- Expected behavior
- Actual behavior
- System information
- Screenshots (if applicable)
- Log files (if available)

## Post-Testing Tasks

- [ ] Document all found issues
- [ ] Verify critical issues are addressed
- [ ] Update test cases based on findings
- [ ] Review test coverage gaps
- [ ] Plan additional testing if needed

## Testing Schedule

### Daily Testing
- Basic functionality verification
- New feature testing
- Critical path testing

### Weekly Testing
- Full regression testing
- Performance testing
- UI/UX review

### Release Testing
- Complete manual testing checklist
- Cross-platform verification
- Performance benchmarking
- Security review
- Documentation verification
