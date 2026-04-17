# API Documentation

This document describes the full interface between the Leptos frontend and the Tauri backend, including all registered commands and emitted events.

## Overview

Communication uses Tauri's command system (`invoke`) for frontend→backend calls and named events (`emit_all` / `listen`) for backend→frontend push. All data crosses the WASM boundary via `serde_wasm_bindgen`.

```rust
// Frontend invocation pattern (non-failing commands)
use crate::app::invoke;  // declared in app.rs with js_namespace ["window","__TAURI__","core"]
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen_futures::spawn_local;

spawn_local(async move {
    let args = to_value(&MyArgs { ... }).unwrap();
    let result = invoke("command_name", args).await;
    let data: MyType = from_value(result).unwrap();
});

// For commands that can return an error, use invoke_catching:
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = "invoke", catch)]
    async fn invoke_catching(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}
```

## Data Models

### `User`

Represents the signed-in user.

```rust
struct User {
    name: String,          // Display name (editable from MainPage)
    email: String,         // Full JID, e.g. "alice@example.com"
    flavour_text: String,  // Status/mood message
    availability: Availability,
}
```

### `Friend`

Represents a contact from the XMPP roster.

```rust
struct Friend {
    name: String,          // Display name (from vCard or roster)
    email: String,         // Full JID, e.g. "bob@example.com"
    flavour_text: String,  // vCard status/mood line
    availability: Availability,
}
```

### `Availability`

```rust
enum Availability { Online, Away, Busy, Offline }
```

### `SavedProfile`

Stored in `%APPDATA%\nto\nto_remembered.json`.

```rust
struct SavedProfile {
    jid: String,
    remember_me: bool,
    auto_sign_in: bool,
    last_availability: String,
}
```

---

## Tauri Commands

All commands are registered in `invoke_handler![]` in `src-tauri/src/main.rs`.

---

### User & Profile

#### `get_user()`

Returns the in-memory user state (populated after `xmpp_set_presence` sets it).

**Returns:** `Result<User, String>`

---

#### `update_username(name: &str)`

Updates the local user's display name in `AppState`.

**Parameters:** `{ name: string }`

**Returns:** `Result<User, String>`

---

#### `get_saved_profile(app_handle)`

Reads `nto_remembered.json` from the OS app-data directory. Returns defaults if the file doesn't exist.

**Returns:** `Result<SavedProfile, String>`

---

#### `save_login_prefs(jid, remember_me, auto_sign_in, last_availability, app_handle)`

Writes login preferences to `nto_remembered.json`. Called after a successful login when "Remember me" is checked.

**Parameters:**
```json
{
  "jid": "alice@example.com",
  "remember_me": true,
  "auto_sign_in": false,
  "last_availability": "Online"
}
```

**Returns:** `Result<(), String>`

---

### Friend Management (Local State)

These operate on the in-memory `AppState.friends` list. The authoritative source is the XMPP roster; these are used for interim updates before the next roster push.

#### `get_friends()`

Returns friends sorted by availability (online/away/busy first, offline last).

**Returns:** `Result<(Vec<Friend>, Vec<Friend>), String>` — tuple `(online, offline)`

---

#### `add_friend(name, email, flavour_text?, availability?)`

Adds a friend to the local list.

**Returns:** `Result<Friend, String>`

---

#### `update_friend(email, name?, flavour_text?, availability?)`

Updates a friend in the local list by JID.

**Returns:** `Result<Friend, String>`

---

### XMPP Connection

#### `xmpp_connect(jid, password, app_handle)`

Establishes a direct-TLS connection on port 5223. Spawns the background event loop. On success emits `xmpp_connected`.

**Parameters:** `{ jid: string, password: string }`

**Returns:**
```json
{ "success": true }
// or
{ "success": false, "error": "reason" }
```

---

#### `xmpp_disconnect(app_handle)`

Sends a disconnect command to the background loop. Emits `xmpp_disconnected`.

**Returns:** `Result<(), String>`

---

#### `xmpp_get_connection_status()`

Polls current connection state synchronously.

**Returns:** `{ "connected": bool, "jid": string | null }`

---

#### `xmpp_register(jid, password, app_handle)`

In-band registration (XEP-0077). Used by the `/register` route.

**Parameters:** `{ jid: string, password: string }`

**Returns:** `{ "success": bool, "error"?: string }`

---

### XMPP Roster

#### `xmpp_request_roster(app_handle)`

Sends a roster IQ. Result arrives as `xmpp_roster_received`. **Must be called on `MainPage` mount** after registering the roster listener to handle the race condition where the post-login roster push fires before the page has mounted.

**Returns:** `Result<(), String>`

---

#### `xmpp_add_contact(jid, app_handle)`

Adds a JID to the roster and sends a subscription request.

**Parameters:** `{ jid: string }`

**Returns:** `Result<(), String>`

---

#### `xmpp_accept_subscription(jid, app_handle)`

Accepts a pending inbound subscription and sends a reciprocal `subscribe`.

**Parameters:** `{ jid: string }`

**Returns:** `Result<(), String>`

---

#### `xmpp_deny_subscription(jid, app_handle)`

Rejects a pending inbound subscription request.

**Parameters:** `{ jid: string }`

**Returns:** `Result<(), String>`

---

#### `get_pending_subscriptions()`

Drains the in-memory buffer of subscription requests that arrived before the `MainPage` listener was registered. Call on mount after `listen("xmpp_subscription_request", ...)`.

**Returns:** `Result<Vec<String>, String>` — list of pending JIDs.

---

### XMPP Messaging

#### `xmpp_send_message(to, body, app_handle)`

Sends a `<message type="chat">` stanza.

**Parameters:** `{ to: string, body: string }`

**Returns:** `Result<(), String>`

---

### XMPP Presence & vCard

#### `xmpp_set_presence(availability, status, app_handle)`

Broadcasts an updated presence stanza and updates `AppState.user`.

**Parameters:** `{ availability: "Online"|"Away"|"Busy"|"Offline", status: string }`

**Returns:** `Result<(), String>`

---

#### `xmpp_fetch_vcard(app_handle)`

Fetches the signed-in user's own vCard. Result arrives as `xmpp_vcard_received`. Call on `MainPage` mount after registering the vCard listener.

**Returns:** `Result<(), String>`

---

#### `xmpp_fetch_contact_vcard(jid, app_handle)`

Fetches a contact's vCard. Result arrives as `xmpp_vcard_received` with the contact's JID.

**Parameters:** `{ jid: string }`

**Returns:** `Result<(), String>`

---

## Tauri Events

Events are emitted via `app_handle.emit_all(...)` and received via `listen(...)` in the frontend. Register ALL listeners before calling any re-fetch command — see Event Timing in `copilot-instructions.md`.

---

### `xmpp_connected`

Fired when the XMPP connection is successfully established.

```json
{ "connected": true, "jid": "alice@example.com", "error": null }
```

Listener: `mainpage_component.rs`

---

### `xmpp_disconnected`

Fired when the connection is lost or explicitly closed.

```json
{ "connected": false, "jid": null, "error": "optional reason" }
```

Listener: `mainpage_component.rs`

---

### `xmpp_roster_received`

Fired in response to `xmpp_request_roster` or the server's initial post-login roster push.

```json
[
  { "jid": "bob@example.com", "name": "Bob", "subscription": "both" },
  { "jid": "carol@example.com", "name": "", "subscription": "to" }
]
```

Listener: `mainpage_component.rs`

---

### `xmpp_roster_push`

Fired when the server sends a live roster update.

```json
{ "jid": "bob@example.com", "name": "Bob", "subscription": "both" }
```

Listener: `mainpage_component.rs`

---

### `xmpp_presence_update`

Fired when a contact's presence changes.

```json
{ "jid": "bob@example.com", "availability": "Online", "status": "Be right back" }
```

Listener: `mainpage_component.rs`

---

### `xmpp_message_received`

Fired when a `<message type="chat">` stanza arrives.

```json
{
  "id": "uuid-string",
  "from": "bob@example.com",
  "to": "alice@example.com",
  "body": "Hello!",
  "timestamp": "2026-04-15T12:00:00Z",
  "message_type": "chat"
}
```

Listener: `chat_component.rs`

---

### `xmpp_subscription_request`

Fired when a contact sends a subscription request. Pre-mount arrivals are buffered and drained via `get_pending_subscriptions`.

```json
{ "from_jid": "carol@example.com" }
```

Listener: `mainpage_component.rs`

---

### `xmpp_vcard_received`

Fired in response to `xmpp_fetch_vcard` or `xmpp_fetch_contact_vcard`.

```json
{ "jid": "alice@example.com", "nickname": "Alice", "flavour_text": "Living the dream" }
```

When `jid` matches the signed-in user, the frontend updates `User.name` and `User.flavour_text`. Otherwise it updates the matching `Friend`.

Listener: `mainpage_component.rs`

---

## Frontend Integration Patterns

### Registering a Tauri Event Listener

```rust
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::closure::Closure;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    async fn listen(event: &str, callback: &js_sys::Function) -> JsValue;
}

spawn_local(async move {
    let cb = Closure::wrap(Box::new(move |event: JsValue| {
        // parse payload and update signals
    }) as Box<dyn Fn(JsValue)>);

    let _ = listen("xmpp_presence_update", cb.as_ref().unchecked_ref()).await;
    cb.forget(); // REQUIRED — keeps the callback alive
});
```

### Mount-time Re-fetch Pattern

Register ALL listeners first, then trigger re-fetches inside the **same** `spawn_local`:

```rust
spawn_local(async move {
    // 1. Register all listeners
    let _ = listen("xmpp_roster_received", roster_cb.as_ref().unchecked_ref()).await;
    let _ = listen("xmpp_vcard_received",  vcard_cb.as_ref().unchecked_ref()).await;

    // 2. Keep callbacks alive
    roster_cb.forget();
    vcard_cb.forget();

    // 3. ONLY NOW trigger re-fetches — listeners guaranteed registered
    let _ = invoke("xmpp_request_roster", JsValue::null()).await;
    let _ = invoke("xmpp_fetch_vcard",    JsValue::null()).await;

    // 4. Drain buffered events
    let pending = invoke("get_pending_subscriptions", JsValue::null()).await;
    // process pending...
});
// ❌ NEVER do re-fetches in a separate spawn_local
```

### Loading Data with `LocalResource`

```rust
let user_resource = LocalResource::new(|| async {
    let raw = invoke("get_user", JsValue::null()).await;
    from_value::<User>(raw).unwrap_or_default()
});
```
