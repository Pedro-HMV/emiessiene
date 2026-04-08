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
    flavour_text: String,
    availability: Availability,
}

#[derive(Serialize, Deserialize, Clone)]
struct Friend {
    name: String,
    email: String,
    flavour_text: String,
    availability: Availability,
}

impl Friend {
    fn new(
        name: String,
        email: String,
        flavour_text: Option<String>,
        availability: Option<Availability>,
    ) -> Self {
        log::info!("Creating new friend: {} <{}>", name, email);
        Self {
            name,
            email,
            flavour_text: flavour_text.unwrap_or("".to_string()),
            availability: availability.unwrap_or(Availability::Online),
        }
    }

    fn update(
        &mut self,
        name: Option<String>,
        flavour_text: Option<String>,
        availability: Option<Availability>,
    ) {
        log::info!("Updating friend: {}", self.email);
        if let Some(name) = name {
            self.name = name;
        }
        if let Some(flavour_text) = flavour_text {
            self.flavour_text = flavour_text;
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
    flavour_text: String,
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

#[derive(Serialize, Deserialize, Clone)]
struct SavedProfile {
    jid: String,
    remember_me: bool,
    auto_sign_in: bool,
    last_availability: String,
}

fn read_profile_data(app_handle: &tauri::AppHandle) -> serde_json::Value {
    let config = app_handle.config();
    let data_dir = match tauri::api::path::app_data_dir(&config) {
        Some(d) => d,
        None => return serde_json::json!({}),
    };
    let file_path = data_dir.join("nto_remembered.json");
    if !file_path.exists() {
        return serde_json::json!({});
    }
    match std::fs::read_to_string(&file_path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|_| serde_json::json!({})),
        Err(_) => serde_json::json!({}),
    }
}

fn write_profile_data(app_handle: &tauri::AppHandle, json: &serde_json::Value) -> Result<(), String> {
    let config = app_handle.config();
    let data_dir = tauri::api::path::app_data_dir(&config)
        .ok_or_else(|| "Failed to get app data directory".to_string())?;
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let file_path = data_dir.join("nto_remembered.json");
    std::fs::write(&file_path, serde_json::to_string(json).unwrap())
        .map_err(|e| e.to_string())
}

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
            flavour_text: f.flavour_text,
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

fn init_state() -> AppState {
    log::info!("Initializing application state");
    let friends = load_friends_list("friends.json").expect("Failed to load friends list");
    let user = User {
        name: "".into(),
        email: "".into(),
        flavour_text: "".into(),
        availability: Availability::Offline,
    };
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
            update_flavour_text,
            update_availability,
            get_saved_jid,
            get_saved_profile,
            save_jid,
            // XMPP commands
            xmpp_connect,
            xmpp_disconnect,
            xmpp_send_message,
            xmpp_set_presence,
            xmpp_add_contact,
            xmpp_get_connection_status,
            xmpp_register,
            save_login_prefs,
            xmpp_fetch_vcard,
            get_pending_subscriptions,
            xmpp_accept_subscription,
            xmpp_deny_subscription,
            xmpp_request_roster,
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
async fn update_username(
    state: App<'_>,
    xmpp_state: State<'_, XmppState>,
    app_handle: tauri::AppHandle,
    name: String,
) -> Result<User, String> {
    log::info!("Updating username to: {}", name);
    let user = {
        let mut app = state.lock().expect("Failed to lock state");
        app.user.name = name.clone();
        app.user.clone()
    };
    // Best-effort vCard update — don't fail if XMPP isn't connected
    let xmpp = xmpp_state.lock().await;
    if xmpp.is_connected().await {
        let desc = user.flavour_text.clone();
        if let Err(e) = xmpp.set_vcard(name, desc).await {
            log::warn!("Failed to update vCard via XMPP: {}", e);
        }
    }
    Ok(user)
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
    flavour_text: Option<String>,
    availability: Option<Availability>,
) -> Result<Friend, String> {
    log::info!("Updating friend: {}", email);
    let mut app = state.lock().expect("Failed to lock state");
    let friend_index = app
        .friends
        .iter()
        .position(|f| f.email == email)
        .ok_or("Friend not found")?;
    app.friends[friend_index].update(name, flavour_text, availability);
    Ok(app.friends[friend_index].clone())
}

#[command]
fn add_friend(
    state: App,
    name: String,
    email: String,
    flavour_text: Option<String>,
    availability: Option<Availability>,
) -> Result<Friend, String> {
    log::info!("Adding new friend: {} <{}>", name, email);
    let mut app = state.lock().expect("Failed to lock state");
    let friend = Friend::new(name, email, flavour_text, availability);
    app.friends.push(friend.clone());
    Ok(friend)
}

#[command]
async fn update_flavour_text(
    state: App<'_>,
    xmpp_state: State<'_, XmppState>,
    app_handle: tauri::AppHandle,
    flavour_text: String,
) -> Result<User, String> {
    log::info!("Updating flavour text");
    let (user, availability) = {
        let mut app = state.lock().expect("Failed to lock state");
        app.user.flavour_text = flavour_text.clone();
        let user = app.user.clone();
        let availability = user.availability.clone();
        (user, availability)
    };
    // In XMPP, persist flavour_text in the vCard DESC field AND broadcast via presence status
    let xmpp = xmpp_state.lock().await;
    if xmpp.is_connected().await {
        let nickname = user.name.clone();
        let desc = flavour_text.clone();
        if let Err(e) = xmpp.set_vcard(nickname, desc).await {
            log::warn!("Failed to update vCard DESC via XMPP: {}", e);
        }
        let status = if flavour_text.is_empty() { None } else { Some(flavour_text) };
        if let Err(e) = xmpp.set_presence(availability, status).await {
            log::warn!("Failed to update XMPP presence status text: {}", e);
        }
    }
    Ok(user)
}

#[command]
async fn update_availability(
    state: App<'_>,
    xmpp_state: State<'_, XmppState>,
    availability: Availability,
) -> Result<User, String> {
    log::info!("Updating availability to: {:?}", availability);
    let (user, flavour_text) = {
        let mut app = state.lock().expect("Failed to lock state");
        app.user.availability = availability.clone();
        let user = app.user.clone();
        let ft = if user.flavour_text.is_empty() { None } else { Some(user.flavour_text.clone()) };
        (user, ft)
    };
    // Send updated XMPP presence (skip for Offline — that's handled by disconnect)
    let xmpp = xmpp_state.lock().await;
    if xmpp.is_connected().await && !matches!(availability, Availability::Offline) {
        if let Err(e) = xmpp.set_presence(availability, flavour_text).await {
            log::warn!("Failed to update XMPP presence availability: {}", e);
        }
    }
    Ok(user)
}

#[command]
fn get_saved_jid(app_handle: tauri::AppHandle) -> Result<String, String> {
    let json = read_profile_data(&app_handle);
    Ok(json["jid"].as_str().unwrap_or("").to_string())
}

#[command]
fn get_saved_profile(app_handle: tauri::AppHandle) -> Result<SavedProfile, String> {
    let json = read_profile_data(&app_handle);
    Ok(SavedProfile {
        jid: json["jid"].as_str().unwrap_or("").to_string(),
        remember_me: json["remember_me"].as_bool().unwrap_or(false),
        auto_sign_in: json["auto_sign_in"].as_bool().unwrap_or(false),
        last_availability: json["last_availability"].as_str().unwrap_or("Online").to_string(),
    })
}

#[command]
fn save_login_prefs(
    app_handle: tauri::AppHandle,
    jid: String,
    remember_me: bool,
    auto_sign_in: bool,
    availability: String,
) -> Result<(), String> {
    let mut json = read_profile_data(&app_handle);
    json["jid"] = serde_json::Value::String(if remember_me { jid } else { String::new() });
    json["remember_me"] = serde_json::Value::Bool(remember_me);
    json["auto_sign_in"] = serde_json::Value::Bool(auto_sign_in);
    json["last_availability"] = serde_json::Value::String(availability);
    write_profile_data(&app_handle, &json)
}

#[command]
fn save_jid(app_handle: tauri::AppHandle, jid: String) -> Result<(), String> {
    // Read-modify-write to preserve existing nickname and flavour_text
    let mut json = read_profile_data(&app_handle);
    json["jid"] = serde_json::Value::String(jid);
    write_profile_data(&app_handle, &json)
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
async fn xmpp_request_roster(xmpp_state: State<'_, XmppState>) -> Result<String, String> {
    log::info!("xmpp_request_roster command called");
    let xmpp_manager = xmpp_state.lock().await;
    xmpp_manager.request_roster().await?;
    Ok("Roster request sent".to_string())
}

#[command]
async fn get_pending_subscriptions(
    xmpp_state: State<'_, XmppState>,
) -> Result<Vec<String>, String> {
    let xmpp_manager = xmpp_state.lock().await;
    Ok(xmpp_manager.drain_pending_subscriptions().await)
}

#[command]
async fn xmpp_fetch_vcard(xmpp_state: State<'_, XmppState>) -> Result<String, String> {
    log::info!("xmpp_fetch_vcard command called");
    let xmpp_manager = xmpp_state.lock().await;
    xmpp_manager.fetch_vcard().await?;
    Ok("vCard request sent".to_string())
}

#[command]
async fn xmpp_accept_subscription(
    xmpp_state: State<'_, XmppState>,
    jid: String,
) -> Result<String, String> {
    log::info!("xmpp_accept_subscription for {}", jid);
    let xmpp_manager = xmpp_state.lock().await;
    xmpp_manager.accept_subscription(jid).await?;
    Ok("Subscription accepted".to_string())
}

#[command]
async fn xmpp_deny_subscription(
    xmpp_state: State<'_, XmppState>,
    jid: String,
) -> Result<String, String> {
    log::info!("xmpp_deny_subscription for {}", jid);
    let xmpp_manager = xmpp_state.lock().await;
    xmpp_manager.deny_subscription(jid).await?;
    Ok("Subscription denied".to_string())
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

// Phase E: In-band Registration (XEP-0077)
// Connects via direct TLS, sends IBR IQ get/set, returns result.
#[command]
async fn xmpp_register(
    jid: String,
    password: String,
) -> Result<serde_json::Value, String> {
    use std::collections::BTreeMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;
    use tokio_xmpp::parsers::ibr::Query as IbrQuery;
    use tokio_xmpp::parsers::iq::Iq as RegIq;

    log::info!("xmpp_register: attempting IBR registration for JID: {}", jid);

    // Parse domain and username from the JID
    let bare_jid: jid::BareJid = jid
        .parse()
        .map_err(|e| format!("Invalid JID '{}': {}", jid, e))?;
    let username = bare_jid
        .node()
        .ok_or("JID must have a username part (user@domain)")?
        .to_string();
    let domain = bare_jid.domain().to_string();

    // Direct-TLS connect (port 5223 – same transport as the main client)
    let tcp = TcpStream::connect(format!("{}:5223", domain))
        .await
        .map_err(|e| format!("TCP connect to {}:5223 failed: {}", domain, e))?;

    let native_connector = {
        let mut builder = native_tls::TlsConnector::builder();
        #[cfg(debug_assertions)]
        builder.danger_accept_invalid_certs(true);
        builder.build().map_err(|e| format!("TLS connector creation failed: {}", e))?
    };
    let connector = tokio_native_tls::TlsConnector::from(native_connector);
    let mut stream = connector
        .connect(&domain, tcp)
        .await
        .map_err(|e| format!("TLS handshake failed: {}", e))?;

    // Open XMPP stream
    let stream_open = format!(
        "<?xml version='1.0'?><stream:stream \
         xmlns='jabber:client' \
         xmlns:stream='http://etherx.jabber.org/streams' \
         to='{}' version='1.0'>",
        domain
    );
    stream
        .write_all(stream_open.as_bytes())
        .await
        .map_err(|e| format!("Stream write error: {}", e))?;

    // Read server greeting + stream features
    let mut buf = vec![0u8; 8192];
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| format!("Stream read error: {}", e))?;
    let features = String::from_utf8_lossy(&buf[..n]).to_string();

    if !features.contains("jabber:iq:register")
        && !features.contains("http://jabber.org/features/iq-register")
    {
        log::warn!("xmpp_register: server {} does not advertise IBR support", domain);
        return Ok(serde_json::json!({
            "success": false,
            "error": "Server does not support in-band registration"
        }));
    }

    // Send IBR IQ get (request available registration fields)
    let get_query = IbrQuery {
        fields: BTreeMap::new(),
        registered: false,
        remove: false,
        form: None,
    };
    let get_iq = RegIq::from_get("reg1", get_query);
    let get_elem = minidom::Element::from(get_iq);
    let mut get_xml = Vec::new();
    get_elem
        .write_to(&mut get_xml)
        .map_err(|e| format!("XML serialization error: {}", e))?;
    stream
        .write_all(&get_xml)
        .await
        .map_err(|e| format!("Stream write error: {}", e))?;

    // Read registration form response
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| format!("Stream read error: {}", e))?;
    let form_resp = String::from_utf8_lossy(&buf[..n]).to_string();
    if form_resp.contains("type='error'") || form_resp.contains("type=\"error\"") {
        return Ok(serde_json::json!({
            "success": false,
            "error": "Server returned error for registration query"
        }));
    }

    // Build and send IBR IQ set with username + password
    // minidom handles XML character escaping for field values
    let mut fields = BTreeMap::new();
    fields.insert("username".to_string(), username);
    fields.insert("password".to_string(), password);
    let set_query = IbrQuery {
        fields,
        registered: false,
        remove: false,
        form: None,
    };
    let set_iq = RegIq::from_set("reg2", set_query);
    let set_elem = minidom::Element::from(set_iq);
    let mut set_xml = Vec::new();
    set_elem
        .write_to(&mut set_xml)
        .map_err(|e| format!("XML serialization error: {}", e))?;
    stream
        .write_all(&set_xml)
        .await
        .map_err(|e| format!("Stream write error: {}", e))?;

    // Read registration result
    let n = stream
        .read(&mut buf)
        .await
        .map_err(|e| format!("Stream read error: {}", e))?;
    let result = String::from_utf8_lossy(&buf[..n]).to_string();

    // Close stream gracefully
    let _ = stream.write_all(b"</stream:stream>").await;

    if result.contains("type='result'") || result.contains("type=\"result\"") {
        log::info!("xmpp_register: registration successful for JID: {}", jid);
        Ok(serde_json::json!({ "success": true }))
    } else if result.contains("conflict") {
        Ok(serde_json::json!({
            "success": false,
            "error": "Username already exists"
        }))
    } else if result.contains("type='error'") || result.contains("type=\"error\"") {
        Ok(serde_json::json!({
            "success": false,
            "error": "Registration failed - check server logs for details"
        }))
    } else {
        log::warn!("xmpp_register: unexpected server response for JID: {}", jid);
        Ok(serde_json::json!({
            "success": false,
            "error": "Unexpected server response during registration"
        }))
    }
}
