// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, fmt::Display, fs::File, io::BufReader, sync::Mutex};

use serde::{Deserialize, Serialize};
use tauri::{command, State};

use log;

mod xmpp_manager;
use xmpp_manager::XmppManager;

type App<'a> = State<'a, Mutex<AppState>>;
// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command

#[derive(Serialize, Deserialize, Clone)]
struct User {
    name: String,
    email: String,
    status: String,
    availability: Availability,
}

#[derive(Serialize, Deserialize, Clone)]
struct UserJson {
    name: String,
    email: String,
    status: String,
    availability: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Friend {
    name: String,
    email: String,
    status: String,
    availability: Availability,
}

impl Friend {
    fn new(
        name: String,
        email: String,
        status: Option<String>,
        availability: Option<Availability>,
    ) -> Self {
        log::info!("Creating new friend: {} <{}>", name, email);
        Self {
            name,
            email,
            status: status.unwrap_or("".to_string()),
            availability: availability.unwrap_or(Availability::Online),
        }
    }

    fn update(
        &mut self,
        name: Option<String>,
        status: Option<String>,
        availability: Option<Availability>,
    ) {
        log::info!("Updating friend: {}", self.email);
        if let Some(name) = name {
            self.name = name;
        }
        if let Some(status) = status {
            self.status = status;
        }
        if let Some(availability) = availability {
            self.availability = availability;
        }
    }
}

#[derive(Serialize, Deserialize)]
struct FriendJson {
    name: String,
    email: String,
    status: String,
    availability: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Availability {
    Online,
    Away,
    Busy,
    Offline,
}

#[derive(Serialize, Deserialize, Clone)]
struct AppState {
    user: User,
    friends: Vec<Friend>,
}

// Separate state for XMPP manager since it contains non-serializable types
type XmppState = std::sync::Arc<tokio::sync::Mutex<XmppManager>>;

impl AppState {
    fn friends_by_availability(&self) -> (Vec<Friend>, Vec<Friend>) {
        log::info!("Sorting friends by availability");
        let mut online_friends = Vec::new();
        let mut offline_friends = Vec::new();
        self.friends.iter().for_each(|f| match f.availability {
            Availability::Offline => offline_friends.push(f.clone()),
            _ => online_friends.push(f.clone()),
        });
        online_friends.sort_by(|a, b| a.email.cmp(&b.email));
        offline_friends.sort_by(|a, b| a.email.cmp(&b.email));
        (online_friends, offline_friends)
    }
}

impl Display for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "User: {}, Friends: {}",
            self.user.name,
            self.friends.len()
        )
    }
}

fn load_friends_list(file_path: &str) -> Result<Vec<Friend>, Box<dyn std::error::Error>> {
    log::info!("Loading friends list from {}", file_path);
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let json: Vec<FriendJson> = serde_json::from_reader(reader)?;

    let friends: Vec<Friend> = json
        .into_iter()
        .map(|f| Friend {
            name: f.name,
            email: f.email,
            status: f.status,
            availability: match f.availability.as_str() {
                "Online" => Availability::Online,
                "Away" => Availability::Away,
                "Busy" => Availability::Busy,
                _ => Availability::Offline,
            },
        })
        .collect();

    log::info!("Loaded {} friends", friends.len());
    Ok(friends)
}

fn load_user(file_path: &str) -> Result<User, Box<dyn std::error::Error>> {
    log::info!("Loading user from {}", file_path);
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    let json: UserJson = serde_json::from_reader(reader)?;

    let user = User {
        name: json.name,
        email: json.email,
        status: json.status,
        availability: match json.availability.as_str() {
            "Online" => Availability::Online,
            "Away" => Availability::Away,
            "Busy" => Availability::Busy,
            _ => Availability::Offline,
        },
    };
    log::info!("Loaded user: {}", user.name);
    Ok(user)
}

fn init_state() -> AppState {
    log::info!("Initializing application state");
    let friends = load_friends_list("friends.json").expect("Failed to load friends list");
    let user = load_user("user.json").expect("Failed to load user");
    AppState { user, friends }
}

fn main() {
    env_logger::init();
    log::info!("Starting application");
    log::info!(
        "Current directory: {}",
        env::current_dir().unwrap().display()
    );
    let app = init_state();
    log::info!("App state: {}", app);

    // Initialize XMPP manager
    let xmpp_manager = XmppManager::new();
    let xmpp_state = std::sync::Arc::new(tokio::sync::Mutex::new(xmpp_manager));

    tauri::Builder::default()
        .manage(Mutex::new(app))
        .manage(xmpp_state)
        .setup(|_app| Ok(()))
        .invoke_handler(tauri::generate_handler![
            get_user,
            get_friends,
            update_friend,
            add_friend,
            update_username,
            // XMPP commands
            xmpp_connect,
            xmpp_disconnect,
            xmpp_send_message,
            xmpp_set_presence,
            xmpp_add_contact,
            xmpp_get_connection_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[command]
fn get_user(state: App) -> Result<User, String> {
    log::info!("Getting user information");
    let app = state.lock().expect("Failed to lock state");
    Ok(app.user.clone())
}

#[command]
fn update_username(state: App, name: String) -> Result<User, String> {
    log::info!("Updating username to: {}", name);
    println!("Updating username to: {}", name);
    let mut app = state.lock().expect("Failed to lock state");
    app.user.name = name;
    Ok(app.user.clone())
}

#[command]
fn get_friends(state: App) -> Result<(Vec<Friend>, Vec<Friend>), String> {
    log::info!("Getting friends list");
    let app = state.lock().expect("Failed to lock state");
    Ok(app.friends_by_availability())
}

#[command]
fn update_friend(
    state: App,
    email: String,
    name: Option<String>,
    status: Option<String>,
    availability: Option<Availability>,
) -> Result<Friend, String> {
    log::info!("Updating friend: {}", email);
    let mut app = state.lock().expect("Failed to lock state");
    let friend_index = app
        .friends
        .iter()
        .position(|f| f.email == email)
        .ok_or("Friend not found")?;
    app.friends[friend_index].update(name, status, availability);
    Ok(app.friends[friend_index].clone())
}

#[command]
fn add_friend(
    state: App,
    name: String,
    email: String,
    status: Option<String>,
    availability: Option<Availability>,
) -> Result<Friend, String> {
    log::info!("Adding new friend: {} <{}>", name, email);
    let mut app = state.lock().expect("Failed to lock state");
    let friend = Friend::new(name, email, status, availability);
    app.friends.push(friend.clone());
    Ok(friend)
}

// XMPP Commands

#[command]
async fn xmpp_connect(
    app_handle: tauri::AppHandle,
    xmpp_state: State<'_, XmppState>,
    jid: String,
    password: String,
) -> Result<serde_json::Value, String> {
    log::info!("XMPP connect command called for JID: {}", jid);
    let mut xmpp_manager = xmpp_state.lock().await;
    xmpp_manager.set_app_handle(app_handle);
    
    match xmpp_manager.connect(jid, password).await {
        Ok(_) => {
            log::info!("XMPP connection successful");
            Ok(serde_json::json!({
                "success": true,
                "message": "Connected successfully"
            }))
        }
        Err(e) => {
            log::error!("XMPP connection failed: {}", e);
            Ok(serde_json::json!({
                "success": false,
                "error": e
            }))
        }
    }
}

#[command]
async fn xmpp_disconnect(xmpp_state: State<'_, XmppState>) -> Result<String, String> {
    log::info!("XMPP disconnect command called");
    let mut xmpp_manager = xmpp_state.lock().await;
    xmpp_manager.disconnect().await?;
    Ok("Disconnected successfully".to_string())
}

#[command]
async fn xmpp_send_message(
    xmpp_state: State<'_, XmppState>,
    to_jid: String,
    body: String,
) -> Result<serde_json::Value, String> {
    log::info!("XMPP send message command called for: {}", to_jid);
    let xmpp_manager = xmpp_state.lock().await;
    
    match xmpp_manager.send_message(to_jid, body).await {
        Ok(_) => {
            log::info!("Message sent successfully via XMPP");
            Ok(serde_json::json!({
                "success": true,
                "message": "Message sent successfully"
            }))
        }
        Err(e) => {
            log::error!("Failed to send XMPP message: {}", e);
            Ok(serde_json::json!({
                "success": false,
                "error": e
            }))
        }
    }
}

#[command]
async fn xmpp_set_presence(
    xmpp_state: State<'_, XmppState>,
    availability: Availability,
    status: Option<String>,
) -> Result<String, String> {
    log::info!("XMPP set presence command called");
    let xmpp_manager = xmpp_state.lock().await;
    xmpp_manager.set_presence(availability, status).await?;
    Ok("Presence updated successfully".to_string())
}

#[command]
async fn xmpp_add_contact(xmpp_state: State<'_, XmppState>, jid: String) -> Result<String, String> {
    log::info!("XMPP add contact command called");
    let xmpp_manager = xmpp_state.lock().await;
    xmpp_manager.add_contact(jid).await?;
    Ok("Contact addition request sent".to_string())
}

#[command]
async fn xmpp_get_connection_status(
    xmpp_state: State<'_, XmppState>,
) -> Result<serde_json::Value, String> {
    let xmpp_manager = xmpp_state.lock().await;
    let connected = xmpp_manager.is_connected().await;
    let jid = xmpp_manager.get_current_jid().await;

    Ok(serde_json::json!({
        "connected": connected,
        "jid": jid
    }))
}
