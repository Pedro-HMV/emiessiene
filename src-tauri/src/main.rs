// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{env, fmt::Display, fs::File, io::BufReader, sync::Mutex};

use serde::{Deserialize, Serialize};
use tauri::{command, State};

use log::{debug, info};

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
        info!("Creating new friend: {} <{}>", name, email);
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
        info!("Updating friend: {}", self.email);
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

#[derive(Serialize, Deserialize, Clone)]
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

impl AppState {
    fn friends_by_availability(&self) -> (Vec<Friend>, Vec<Friend>) {
        info!("Sorting friends by availability");
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
    info!("Loading friends list from {}", file_path);
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

    info!("Loaded {} friends", friends.len());
    Ok(friends)
}

fn load_user(file_path: &str) -> Result<User, Box<dyn std::error::Error>> {
    info!("Loading user from {}", file_path);
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
    info!("Loaded user: {}", user.name);
    Ok(user)
}

fn init_state() -> AppState {
    info!("Initializing application state");
    let friends = load_friends_list("friends.json").expect("Failed to load friends list");
    let user = load_user("user.json").expect("Failed to load user");
    AppState { user, friends }
}

fn main() {
    env_logger::init();
    info!("Starting application");
    info!(
        "Currect directory: {}",
        env::current_dir().unwrap().display()
    );
    let app = init_state();
    info!("App state: {}", app);
    tauri::Builder::default()
        .manage(Mutex::new(app))
        .invoke_handler(tauri::generate_handler![
            get_user,
            get_friends,
            update_friend,
            add_friend,
            update_username,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[command]
fn get_user(state: App) -> Result<User, String> {
    info!("Getting user information");
    let app = state.lock().expect("Failed to lock state");
    Ok(app.user.clone())
}

#[command]
fn update_username(state: App, name: String) -> Result<User, String> {
    info!("Updating username to: {}", name);
    println!("Updating username to: {}", name);
    let mut app = state.lock().expect("Failed to lock state");
    app.user.name = name;
    Ok(app.user.clone())
}

#[command]
fn get_friends(state: App) -> Result<(Vec<Friend>, Vec<Friend>), String> {
    info!("Getting friends list");
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
    info!("Updating friend: {}", email);
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
    info!("Adding new friend: {} <{}>", name, email);
    let mut app = state.lock().expect("Failed to lock state");
    let friend = Friend::new(name, email, status, availability);
    app.friends.push(friend.clone());
    Ok(friend)
}
