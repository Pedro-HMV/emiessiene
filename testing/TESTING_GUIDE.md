# NTO Testing Guide

## Test Credentials & User Creation

### Quick Testing (Predefined Users)

Since XMPP is currently simulated, **any credentials will work**. However, here are some predefined test users for consistency:

#### Primary Test User
- **Email/JID**: `pedro@hotmail.com`
- **Password**: `any password` (e.g., `test123`)
- **Name**: Pedro
- **Status**: "Tá saindo da jaula o MSNtro!"

#### Additional Test Users
- **Email**: `juniorbcm@hotmail.com` (Death Scyther)
- **Email**: `giovanni_p@hotmail.com` (Burega The King)  
- **Email**: `galaxyblues@hotmail.com` (Sawamura Shido)
- **Email**: `ivokds@hotmail.com` (Kushirenada)

### User Registration (New Feature)

You can now create new test accounts using the registration system:

1. **Access Registration**: Click "Create Account" on the login page or go to `/register`
2. **Fill Registration Form**:
   - **Full Name**: Your display name
   - **Email**: Use format like `your.name@domain.com`
   - **Password**: Any password (currently not validated)
   - **Status Message**: Your initial status
   - **Initial Status**: Online/Away/Busy/Offline

3. **Complete Registration**: Click "Create Account" (currently simulated)

## Testing Workflow

### 1. Login Flow
```
1. Open application at http://localhost:1420/
2. Enter email (e.g., pedro@hotmail.com)
3. Enter any password
4. Select availability status
5. Click "Sign In"
6. Should navigate to main page with "🟢 Connected" status
```

### 2. XMPP Integration Testing
```
1. Login successfully
2. Check connection status in main page header
3. Open chat with a friend
4. Send messages (should show in chat)
5. Messages are logged in console for verification
```

### 3. Navigation Testing
```
1. Login page (/) ✓
2. Registration page (/register) ✓
3. Main page (/main) ✓  
4. Chat page (/chat/0, /chat/1, etc.) ✓
5. Sign out (returns to login) ✓
```

## Expected Behaviors

### ✅ Working Features
- [x] User login with any credentials
- [x] User registration form (UI complete)
- [x] XMPP connection simulation
- [x] Real-time connection status display
- [x] Chat interface with message sending
- [x] Friends list display (online/offline)
- [x] Navigation between pages
- [x] Responsive signal management

### 🔄 Simulated Features
- **XMPP Connection**: Always succeeds
- **Message Sending**: Logs to console, simulates success
- **User Registration**: Form validation, no backend storage yet
- **Friend Presence**: Static from JSON files

### 🚧 Future Enhancements
- Real XMPP server integration
- User account persistence
- Dynamic friend management
- Real-time message delivery
- File transfer capabilities

## Troubleshooting

### Common Issues
1. **"invoke_xmpp is not a function"**: Fixed ✅
2. **"missing required key jid"**: Fixed ✅
3. **Reactive signal disposal**: Fixed ✅
4. **Build compilation errors**: Fixed ✅

### Debug Information
- Check browser console for XMPP events
- Backend logs show connection attempts
- Frontend logs show message sending attempts

## Advanced Testing

### Custom User Creation
You can modify the JSON files directly for more test users:

1. **Edit**: `src-tauri/user.json` for main user
2. **Edit**: `src-tauri/friends.json` for friends list
3. **Restart**: Application to reload changes

### XMPP Event Testing
Open browser dev tools and watch for:
- `xmpp_connected` events
- `xmpp_message_received` events  
- `xmpp_disconnected` events

The application is ready for comprehensive testing! 🎉
