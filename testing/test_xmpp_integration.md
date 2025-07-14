# XMPP Integration Testing Guide

## Overview
This document provides instructions for testing the XMPP integration in the NTO application.

## Available XMPP Commands

The following Tauri commands are now available for XMPP functionality:

### 1. `xmpp_connect`
```javascript
// Connect to XMPP server
await invoke('xmpp_connect', {
    jid: 'user@nto.local',
    password: 'password'
});
```

### 2. `xmpp_send_message`
```javascript
// Send a message
await invoke('xmpp_send_message', {
    toJid: 'friend@nto.local',
    body: 'Hello, this is a test message!'
});
```

### 3. `xmpp_set_presence`
```javascript
// Set user presence
await invoke('xmpp_set_presence', {
    availability: 'Online', // Online, Away, Busy, Offline
    status: 'Available for chat'
});
```

### 4. `xmpp_add_contact`
```javascript
// Add a contact
await invoke('xmpp_add_contact', {
    jid: 'newFriend@nto.local'
});
```

### 5. `xmpp_disconnect`
```javascript
// Disconnect from XMPP server
await invoke('xmpp_disconnect');
```

### 6. `xmpp_get_connection_status`
```javascript
// Get current connection status
const status = await invoke('xmpp_get_connection_status');
console.log(status); // { connected: true, jid: "user@nto.local" }
```

## Event Listeners

The XMPP manager emits the following events that the frontend can listen to:

### Connection Events
```javascript
// Listen for connection events
listen('xmpp_connected', (event) => {
    console.log('Connected:', event.payload);
});

listen('xmpp_disconnected', (event) => {
    console.log('Disconnected:', event.payload);
});
```

### Message Events
```javascript
// Listen for message events
listen('xmpp_message_sent', (event) => {
    console.log('Message sent:', event.payload);
});

listen('xmpp_message_received', (event) => {
    console.log('Message received:', event.payload);
});
```

### Presence Events
```javascript
// Listen for presence updates
listen('xmpp_presence_updated', (event) => {
    console.log('Presence updated:', event.payload);
});
```

### Contact Events
```javascript
// Listen for contact events
listen('xmpp_contact_added', (event) => {
    console.log('Contact added:', event.payload);
});
```

## Testing Steps

### Manual Testing with Browser Console

1. **Start the application**: `cargo tauri dev`
2. **Open browser console** in the dev window
3. **Test connection**:
   ```javascript
   // First import the invoke function
   const { invoke } = window.__TAURI__.tauri;
   
   // Test connection
   invoke('xmpp_connect', {
       jid: 'testuser@nto.local',
       password: 'testpass'
   }).then(result => {
       console.log('Connect result:', result);
   }).catch(err => {
       console.error('Connect error:', err);
   });
   ```

4. **Test sending a message**:
   ```javascript
   invoke('xmpp_send_message', {
       toJid: 'friend@nto.local',
       body: 'Hello from browser console!'
   }).then(result => {
       console.log('Send message result:', result);
   });
   ```

5. **Test presence update**:
   ```javascript
   invoke('xmpp_set_presence', {
       availability: 'Away',
       status: 'Testing presence from console'
   }).then(result => {
       console.log('Set presence result:', result);
   });
   ```

### Event Listener Testing

```javascript
// Set up event listeners
const { listen } = window.__TAURI__.event;

listen('xmpp_connected', (event) => {
    console.log('🟢 XMPP Connected:', event.payload);
});

listen('xmpp_message_sent', (event) => {
    console.log('📤 Message Sent:', event.payload);
});

listen('xmpp_presence_updated', (event) => {
    console.log('👤 Presence Updated:', event.payload);
});
```

## Current Implementation Status

### ✅ Completed
- Basic XMPP manager structure
- All command interfaces implemented
- Event emission system
- Simulated XMPP operations
- Tauri integration complete

### 🚧 In Progress
- Real XMPP protocol implementation (currently simulated)
- Connection to actual Prosody server

### 📋 TODO
- Replace simulated operations with real XMPP client
- Add proper error handling for network issues
- Implement message history persistence
- Add support for file transfers
- Implement group chat functionality

## Development Notes

1. **Simulation Mode**: The current implementation simulates XMPP operations for development purposes
2. **Event System**: Uses Tauri's event system for real-time communication between backend and frontend
3. **Async Operations**: All XMPP operations are asynchronous and return promises
4. **Error Handling**: Comprehensive error messages for debugging

## Next Steps

1. **Integrate with Frontend**: Update login and chat components to use XMPP commands
2. **Real XMPP Implementation**: Replace simulation with actual tokio-xmpp client
3. **UI Updates**: Add connection status indicators and error handling in the UI
4. **Testing**: Create automated tests for XMPP functionality

## Example Integration

Here's how to integrate XMPP into the existing login component:

```rust
// In login component
async fn handle_login() {
    let jid = format!("{}@nto.local", username);
    
    match invoke("xmpp_connect", &serde_json::json!({
        "jid": jid,
        "password": password
    })).await {
        Ok(_) => {
            // Connection successful, navigate to main page
            navigate("/main", Default::default());
        }
        Err(e) => {
            // Show error message
            set_error_message(format!("Login failed: {}", e));
        }
    }
}
```

This integration foundation provides a solid base for implementing real-time XMPP messaging in the NTO application.
