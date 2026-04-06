use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize)]
pub struct UpdateUsernameArgs<'a> {
    pub name: &'a str,
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
