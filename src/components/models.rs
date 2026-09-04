use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;

/// The XMPP server address. Usernames entered in the UI are combined with
/// this to form a full JID (e.g. "alice" → "alice@203.0.113.42").
pub const XMPP_SERVER: &str = "203.0.113.42";

#[derive(Serialize, Deserialize)]
pub struct UpdateUsernameArgs<'a> {
    pub name: &'a str,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SavedProfile {
    pub jid: String,
    pub remember_me: bool,
    pub auto_sign_in: bool,
    pub last_availability: String,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateFlavourTextArgs<'a> {
    #[serde(rename = "flavourText")]
    pub flavour_text: &'a str,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateAvailabilityArgs {
    pub availability: Availability,
}

/// A single chat message, stored in the global per-conversation history.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Bare JID of the sender (e.g. "alice@localhost"). For self-sent messages this is our own JID.
    pub from_jid: String,
    pub body: String,
    /// ISO-8601 timestamp string as received from the backend.
    pub timestamp: String,
    /// true if we sent this message.
    pub is_self: bool,
    /// true until the backend confirms the send succeeded.
    pub pending: bool,
}

/// Global message store: bare JID → ordered conversation history.
pub type MessageStore = HashMap<String, Vec<ChatMessage>>;

/// Args for the `xmpp_send_message` Tauri command.
/// Tauri v1 maps snake_case parameter names to camelCase on the wire.
#[derive(Serialize)]
pub struct XmppSendMessageArgs {
    #[serde(rename = "toJid")]
    pub to_jid: String,
    pub body: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct User {
    pub name: String,
    pub email: String,
    pub flavour_text: String,
    pub availability: Availability,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Friend {
    pub name: String,
    pub email: String,
    pub flavour_text: String,
    pub availability: Availability,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Availability {
    Online,
    Away,
    Busy,
    Offline,
}

impl Display for Availability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Availability::Online => write!(f, "Online"),
            Availability::Away => write!(f, "Away"),
            Availability::Busy => write!(f, "Busy"),
            Availability::Offline => write!(f, "Offline"),
        }
    }
}

impl Availability {
    pub fn to_icon(&self) -> String {
        match self {
            Availability::Online => "👤".to_string(),
            Availability::Away => "⏳".to_string(),
            Availability::Busy => "⛔".to_string(),
            Availability::Offline => "📴".to_string(),
        }
    }
}
