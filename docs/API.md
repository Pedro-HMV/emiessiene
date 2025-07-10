# API Documentation

This document describes the API interface between the Leptos frontend and Tauri backend.

## Overview

The application uses Tauri's command system to communicate between the frontend (WASM) and backend (native Rust). All communication is type-safe and uses Serde for serialization.

## Data Models

### User

Represents the current application user.

```rust
#[derive(Serialize, Deserialize, Clone)]
struct User {
    name: String,        // Display name
    email: String,       // Email address (unique identifier)
    status: String,      // Custom status message
    availability: Availability,  // Current availability status
}
```

**Example:**
```json
{
    "name": "Pedro",
    "email": "pedro@hotmail.com",
    "status": "Tá saindo da jaula o MSNtro!",
    "availability": "Online"
}
```

### Friend

Represents a friend in the user's contact list.

```rust
#[derive(Serialize, Deserialize, Clone)]
struct Friend {
    name: String,        // Display name
    email: String,       // Email address (unique identifier)
    status: String,      // Custom status message
    availability: Availability,  // Current availability status
}
```

**Example:**
```json
{
    "name": "Death Scyther",
    "email": "juniorbcm@hotmail.com",
    "status": "bRO e baixaria!",
    "availability": "Online"
}
```

### Availability

Enumeration of possible user availability states.

```rust
#[derive(Serialize, Deserialize, Clone)]
enum Availability {
    Online,   // Available for chat
    Away,     // Away from computer
    Busy,     // Do not disturb
    Offline,  // Not available
}
```

### UpdateUsernameArgs

Parameter structure for username update requests.

```rust
#[derive(Serialize, Deserialize)]
struct UpdateUsernameArgs<'a> {
    name: &'a str,  // New username
}
```

## API Commands

### User Management

#### `get_user()`

Retrieves the current user information.

**Parameters:** None

**Returns:** `Result<User, String>`

**Example Usage:**
```rust
// Frontend (Leptos)
let user_info = invoke("get_user", JsValue::null()).await;
let user: User = from_value(user_info).expect("Failed to parse user");
```

**Success Response:**
```json
{
    "name": "Pedro",
    "email": "pedro@hotmail.com",
    "status": "Tá saindo da jaula o MSNtro!",
    "availability": "Online"
}
```

**Error Response:**
```json
"Failed to load user data"
```

---

#### `update_username(name: String)`

Updates the current user's display name.

**Parameters:**
- `name: String` - The new display name

**Returns:** `Result<User, String>`

**Example Usage:**
```rust
// Frontend (Leptos)
let args = UpdateUsernameArgs { name: "New Name" };
let result = invoke("update_username", to_value(&args).unwrap()).await;
let updated_user: User = from_value(result).expect("Failed to parse updated user");
```

**Success Response:**
```json
{
    "name": "New Name",
    "email": "pedro@hotmail.com",
    "status": "Tá saindo da jaula o MSNtro!",
    "availability": "Online"
}
```

**Error Response:**
```json
"Failed to update username"
```

### Friend Management

#### `get_friends()`

Retrieves the friends list, sorted by availability (online first, then offline).

**Parameters:** None

**Returns:** `Result<(Vec<Friend>, Vec<Friend>), String>`

The return value is a tuple where:
- First element: Vector of online friends (Online, Away, Busy)
- Second element: Vector of offline friends

**Example Usage:**
```rust
// Frontend (Leptos)
let friends_info = invoke("get_friends", JsValue::null()).await;
let (online_friends, offline_friends): (Vec<Friend>, Vec<Friend>) = 
    from_value(friends_info).expect("Failed to parse friends");
```

**Success Response:**
```json
[
    [
        {
            "name": "Death Scyther",
            "email": "juniorbcm@hotmail.com",
            "status": "bRO e baixaria!",
            "availability": "Online"
        },
        {
            "name": "Burega The King",
            "email": "giovanni_p@hotmail.com",
            "status": "Freeeedommm!",
            "availability": "Away"
        }
    ],
    [
        {
            "name": "Offline Friend",
            "email": "offline@hotmail.com",
            "status": "Last seen yesterday",
            "availability": "Offline"
        }
    ]
]
```

**Error Response:**
```json
"Failed to load friends list"
```

---

#### `add_friend(name: String, email: String, status: Option<String>, availability: Option<Availability>)`

Adds a new friend to the contact list.

**Parameters:**
- `name: String` - Friend's display name
- `email: String` - Friend's email address (must be unique)
- `status: Option<String>` - Optional status message (defaults to empty string)
- `availability: Option<Availability>` - Optional availability (defaults to Online)

**Returns:** `Result<Friend, String>`

**Example Usage:**
```rust
// Frontend (Leptos)
let args = serde_json::json!({
    "name": "New Friend",
    "email": "newfriend@hotmail.com",
    "status": "Hello there!",
    "availability": "Online"
});
let result = invoke("add_friend", to_value(&args).unwrap()).await;
let new_friend: Friend = from_value(result).expect("Failed to parse new friend");
```

**Success Response:**
```json
{
    "name": "New Friend",
    "email": "newfriend@hotmail.com",
    "status": "Hello there!",
    "availability": "Online"
}
```

**Error Responses:**
```json
"Friend with this email already exists"
```
```json
"Invalid email format"
```

---

#### `update_friend(email: String, name: Option<String>, status: Option<String>, availability: Option<Availability>)`

Updates an existing friend's information.

**Parameters:**
- `email: String` - Friend's email address (identifier)
- `name: Option<String>` - Optional new display name
- `status: Option<String>` - Optional new status message
- `availability: Option<Availability>` - Optional new availability

**Returns:** `Result<Friend, String>`

**Example Usage:**
```rust
// Frontend (Leptos)
let args = serde_json::json!({
    "email": "friend@hotmail.com",
    "name": "Updated Name",
    "status": "New status message",
    "availability": "Away"
});
let result = invoke("update_friend", to_value(&args).unwrap()).await;
let updated_friend: Friend = from_value(result).expect("Failed to parse updated friend");
```

**Success Response:**
```json
{
    "name": "Updated Name",
    "email": "friend@hotmail.com",
    "status": "New status message",
    "availability": "Away"
}
```

**Error Response:**
```json
"Friend not found"
```

## Frontend Integration

### Using API Commands in Leptos

#### Basic Command Invocation

```rust
use crate::app::invoke;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::JsValue;

// Simple command without parameters
let result = invoke("get_user", JsValue::null()).await;

// Command with parameters
let params = to_value(&some_struct).unwrap();
let result = invoke("command_name", params).await;

// Parse result
let data: DataType = from_value(result).expect("Failed to parse response");
```

#### Using with Leptos Resources

```rust
// Load data reactively
let user_resource = LocalResource::new(|| async {
    let info = invoke("get_user", JsValue::null()).await;
    from_value::<User>(info).expect("Failed to parse user info")
});

// Use in component
view! {
    <div>
        {move || match user_resource.get() {
            Some(user) => view! { <span>{user.name}</span> }.into_view(),
            None => view! { <span>"Loading..."</span> }.into_view(),
        }}
    </div>
}
```

#### Using with Leptos Actions

```rust
// Create action for user input
let update_user_action = Action::new(|name: &String| {
    let name = name.clone();
    async move {
        let args = UpdateUsernameArgs { name: &name };
        let result = invoke("update_username", to_value(&args).unwrap()).await;
        from_value::<User>(result).expect("Failed to update user")
    }
});

// Trigger action
let handle_submit = move |new_name: String| {
    update_user_action.dispatch(new_name);
};
```

## Error Handling

### Backend Error Patterns

All commands return `Result<T, String>` where the error string provides a human-readable error message.

Common error patterns:
- Data not found: `"User not found"`, `"Friend not found"`
- Validation errors: `"Invalid email format"`, `"Name cannot be empty"`
- System errors: `"Failed to load data"`, `"Failed to save changes"`

### Frontend Error Handling

```rust
// Handle errors in async operations
spawn_local(async move {
    match invoke("some_command", params).await {
        Ok(result) => {
            // Handle success
            let data: DataType = from_value(result).unwrap();
            // Process data
        }
        Err(error) => {
            // Handle error
            logging::error!("Command failed: {:?}", error);
            // Show user feedback
        }
    }
});
```

## Data Storage

### Current Implementation

The backend currently uses JSON files for data storage:

- `src-tauri/user.json` - User information
- `src-tauri/friends.json` - Friends list

### File Formats

**user.json:**
```json
{
    "name": "Pedro",
    "email": "pedro@hotmail.com",
    "status": "Tá saindo da jaula o MSNtro!",
    "availability": "Online"
}
```

**friends.json:**
```json
[
    {
        "name": "Death Scyther",
        "email": "juniorbcm@hotmail.com",
        "status": "bRO e baixaria!",
        "availability": "Online"
    },
    {
        "name": "Burega The King",
        "email": "giovanni_p@hotmail.com",
        "status": "Freeeedommm!",
        "availability": "Online"
    }
]
```

### Future Considerations

For production use, consider:
- Database integration (SQLite, PostgreSQL)
- Encrypted storage for sensitive data
- Backup and synchronization mechanisms
- Migration scripts for data format changes

## Security Considerations

### Input Validation

- All string inputs are validated for length and content
- Email addresses are validated for proper format
- Availability values are restricted to enum variants

### Data Sanitization

- User inputs are sanitized to prevent injection attacks
- File paths are validated to prevent directory traversal
- Error messages don't expose internal system details

### Tauri Security

- Commands are explicitly whitelisted in the Tauri configuration
- Frontend-backend communication is type-safe and validated
- File system access is restricted to application directory

## Testing the API

### Manual Testing

Use the frontend interface to test API functionality:

1. **User Operations**
   - Login and verify user data loads
   - Edit username and verify updates
   - Check status message display

2. **Friend Operations**
   - View friends list and verify sorting
   - Add new friends (if UI exists)
   - Update friend information (if UI exists)

### Automated Testing

```rust
// Backend unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_operations() {
        // Test user-related functions
    }

    #[test]
    fn test_friend_operations() {
        // Test friend-related functions
    }
}
```

## Performance Notes

- Commands are lightweight and typically complete in < 10ms
- JSON file operations are synchronous and may block for large datasets
- Consider caching for frequently accessed data
- Batch operations for multiple friend updates

## Version Compatibility

This API documentation is for version 0.1.0 of the EmiEssiEne application.

Future versions may include:
- Additional user profile fields
- Group chat support
- Message history storage
- Real-time synchronization
- Enhanced status options
