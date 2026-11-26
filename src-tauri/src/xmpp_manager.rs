use futures::StreamExt;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::Emitter;
use tokio::sync::{mpsc, Mutex};
use tokio_xmpp::minidom::Element as MinidomElement;
use tokio_xmpp::parsers::iq::Iq as XmppIq;
use tokio_xmpp::parsers::message::{Lang, Message as XmppMsg, MessageType};
use tokio_xmpp::parsers::presence::{Presence, Show, Type as PresenceType};
use tokio_xmpp::parsers::roster::{Ask, Item as RosterItem, Roster, Subscription};
use tokio_xmpp::{Event, Stanza};

// Re-export for use in main.rs
pub use crate::Availability;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct XmppMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub body: String,
    pub timestamp: String,
    pub message_type: String,
    pub is_self: bool,
    pub group_room: Option<String>,
    pub author_nickname: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct XmppPresence {
    pub jid: String,
    pub availability: Availability,
    pub status: Option<String>,
}

type PresenceCache = HashMap<String, HashMap<String, XmppPresence>>;

fn update_presence_resource(
    cache: &mut PresenceCache,
    full_jid: &str,
    availability: Availability,
    status: Option<String>,
) -> XmppPresence {
    let bare_jid = full_jid.split('/').next().unwrap_or(full_jid).to_string();
    let resource = full_jid
        .split('/')
        .nth(1)
        .filter(|resource| !resource.is_empty())
        .unwrap_or("bare")
        .to_string();

    let resources = cache.entry(bare_jid.clone()).or_default();
    if matches!(availability, Availability::Offline) {
        resources.remove(&resource);
    } else {
        resources.insert(
            resource,
            XmppPresence {
                jid: bare_jid.clone(),
                availability: availability.clone(),
                status: status.clone(),
            },
        );
    }

    let aggregate = aggregate_presence(&bare_jid, resources);
    if matches!(aggregate.availability, Availability::Offline) {
        cache.remove(&bare_jid);
    }
    aggregate
}

fn aggregate_presence(bare_jid: &str, resources: &HashMap<String, XmppPresence>) -> XmppPresence {
    let mut selected: Option<&XmppPresence> = None;
    for presence in resources.values() {
        let replace = match selected {
            None => true,
            Some(current) => {
                presence_priority(&presence.availability) > presence_priority(&current.availability)
            }
        };
        if replace {
            selected = Some(presence);
        }
    }

    selected.cloned().unwrap_or_else(|| XmppPresence {
        jid: bare_jid.to_string(),
        availability: Availability::Offline,
        status: None,
    })
}

fn presence_priority(availability: &Availability) -> u8 {
    match availability {
        Availability::Busy => 3,
        Availability::Away => 2,
        Availability::Online => 1,
        Availability::Offline => 0,
    }
}

fn bare_xmpp_jid(full_jid: &str) -> String {
    full_jid.split('/').next().unwrap_or(full_jid).to_string()
}

fn live_individual_is_self(from_full: &str, own_full: &str) -> bool {
    bare_xmpp_jid(from_full) == bare_xmpp_jid(own_full)
}

fn muc_room_jid(from_full: &str) -> String {
    bare_xmpp_jid(from_full)
}

fn muc_room_from_jids(from_full: &str, to_full: &str) -> Option<String> {
    let from_bare = bare_xmpp_jid(from_full);
    let to_bare = bare_xmpp_jid(to_full);
    if from_bare.contains("@conference.") {
        Some(from_bare)
    } else if to_bare.contains("@conference.") {
        Some(to_bare)
    } else {
        None
    }
}

fn mam_peer_jid(
    from_full: &str,
    to_full: &str,
    own_full: &str,
    is_groupchat: bool,
) -> (String, bool) {
    let from_bare = bare_xmpp_jid(from_full);
    let to_bare = bare_xmpp_jid(to_full);
    if is_groupchat {
        let peer = muc_room_from_jids(from_full, to_full).unwrap_or_else(|| from_bare.clone());
        let own_nickname = own_full
            .split('/')
            .next()
            .unwrap_or(own_full)
            .split('@')
            .next()
            .unwrap_or("")
            .to_string();
        let archived_nickname = from_full.split('/').nth(1).unwrap_or("").to_string();
        (peer, archived_nickname == own_nickname)
    } else {
        let is_self = live_individual_is_self(from_full, own_full);
        let peer = if is_self { to_bare } else { from_bare };
        (peer, is_self)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct XmppGroupInvite {
    pub room_jid: String,
    pub room_name: String,
    pub inviter: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub jid: Option<String>,
    pub error: Option<String>,
}

enum OutgoingCmd {
    SendMessage {
        to: String,
        body: String,
    },
    SendNudge {
        to: String,
    },
    SendGroupMessage {
        room_jid: String,
        body: String,
    },
    JoinGroup {
        room_jid: String,
        nickname: String,
    },
    LeaveGroup {
        room_jid: String,
        nickname: String,
    },
    SendGroupInvite {
        room_jid: String,
        room_name: String,
        inviter: String,
        invitee: String,
    },
    SetGroupSubject {
        room_jid: String,
        subject: String,
    },
    RemoveGroupMember {
        room_jid: String,
        member_jid: String,
    },
    SetPresence {
        show: Option<String>,
        status_text: Option<String>,
        unavailable: bool,
    },
    BroadcastPresence,
    SetVcard {
        nickname: String,
        desc: String,
    },
    FetchVcard,
    FetchContactVcard {
        jid: String,
    },
    RequestRoster,
    FetchHistory,
    FetchGroupHistory {
        room_jid: String,
    },
    AddContact {
        jid: String,
    },
    SendPresenceDirected {
        to: String,
        pres_type: String,
    },
    AcceptAndSubscribe {
        jid: String,
    },
    Disconnect,
}

struct XmppEventLoopState {
    status: Arc<Mutex<ConnectionStatus>>,
    own_jid: String,
    pending_subscriptions: Arc<Mutex<Vec<String>>>,
    pending_group_invites: Arc<Mutex<Vec<XmppGroupInvite>>>,
    presence_cache: Arc<Mutex<PresenceCache>>,
    current_show: Arc<Mutex<Option<String>>>,
}

mod configured_connector {
    use sasl::common::ChannelBinding;
    use std::borrow::Cow;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use tokio::io::BufStream;
    use tokio::net::TcpStream;
    use tokio_xmpp::{
        connect::ServerConnector,
        error::Error as XmppError,
        xmlstream::{initiate_stream, PendingFeaturesRecv, StreamHeader, Timeouts},
    };

    #[derive(Clone, Debug)]
    pub struct ConfiguredTlsConnector {
        pub host: String,
        pub port: u16,
        pub allow_invalid_certs: bool,
        /// Set to true after an auth failure to silently halt tokio-xmpp's internal
        /// exponential-backoff retry loop (which runs in a detached task we can't cancel).
        pub stop: Arc<AtomicBool>,
    }

    impl ServerConnector for ConfiguredTlsConnector {
        type Stream = BufStream<tokio_native_tls::TlsStream<TcpStream>>;

        async fn connect(
            &self,
            jid: &jid::Jid,
            ns: &'static str,
            timeouts: Timeouts,
        ) -> Result<(PendingFeaturesRecv<Self::Stream>, ChannelBinding), XmppError> {
            // If flagged, suspend this task forever — stops the retry loop dead without
            // crashing or spamming logs. The task leaks until the app closes (benign).
            if self.stop.load(Ordering::Relaxed) {
                std::future::pending::<()>().await;
            }
            // TCP connect
            let tcp = TcpStream::connect((self.host.as_str(), self.port))
                .await
                .map_err(XmppError::Io)?;

            // Certificate validation is disabled only when explicitly enabled by
            // NTO_XMPP_ALLOW_INVALID_CERTS in a debug build.
            let native_connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(self.allow_invalid_certs)
                .build()
                .map_err(|e| XmppError::Io(std::io::Error::other(e.to_string())))?;
            let tokio_connector = tokio_native_tls::TlsConnector::from(native_connector);
            let tls_stream = tokio_connector
                .connect(jid.domain().as_str(), tcp)
                .await
                .map_err(|e| XmppError::Io(std::io::Error::other(e.to_string())))?;

            // XMPP stream initiation — Cow::Owned avoids borrow lifetime issues
            let domain = jid.domain().to_string();
            let bare_jid = jid.to_bare().to_string();
            let pending = initiate_stream(
                BufStream::new(tls_stream),
                ns,
                StreamHeader {
                    to: Some(Cow::Owned(domain)),
                    from: Some(Cow::Owned(bare_jid)),
                    id: None,
                },
                timeouts,
            )
            .await
            .map_err(XmppError::Io)?;

            Ok((pending, ChannelBinding::None))
        }
    }
}

pub(crate) fn xmpp_port() -> u16 {
    std::env::var("NTO_XMPP_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|port| *port > 0)
        .unwrap_or(5223)
}

pub(crate) fn xmpp_allow_invalid_certs() -> bool {
    if !cfg!(debug_assertions) {
        return false;
    }

    !matches!(
        std::env::var("NTO_XMPP_ALLOW_INVALID_CERTS")
            .ok()
            .as_deref(),
        Some("0" | "false" | "FALSE" | "no")
    )
}

pub struct XmppManager {
    sender: Option<mpsc::Sender<OutgoingCmd>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
    app_handle: Option<tauri::AppHandle>,
    pending_subscriptions: Arc<Mutex<Vec<String>>>,
    pending_group_invites: Arc<Mutex<Vec<XmppGroupInvite>>>,
    /// Cache of last-known presence per bare JID. Populated by the event loop,
    /// read by `xmpp_get_presence_cache` so MainPage can replay on mount.
    presence_cache: Arc<Mutex<PresenceCache>>,
    /// Last `show` value sent (None = available/online). Used by BroadcastPresence
    /// to re-send the correct show value after a vCard change.
    current_show: Arc<Mutex<Option<String>>>,
}

impl XmppManager {
    pub fn new() -> Self {
        info!("Creating new XMPP Manager");
        Self {
            sender: None,
            task_handle: None,
            connection_status: Arc::new(Mutex::new(ConnectionStatus {
                connected: false,
                jid: None,
                error: None,
            })),
            app_handle: None,
            pending_subscriptions: Arc::new(Mutex::new(Vec::new())),
            pending_group_invites: Arc::new(Mutex::new(Vec::new())),
            presence_cache: Arc::new(Mutex::new(HashMap::new())),
            current_show: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_app_handle(&mut self, app_handle: tauri::AppHandle) {
        self.app_handle = Some(app_handle);
    }

    pub async fn connect(&mut self, jid_str: String, password: String) -> Result<(), String> {
        info!("Attempting XMPP connection for JID: {}", jid_str);
        self.presence_cache.lock().await.clear();

        // Disconnect any existing connection first
        if self.sender.is_some() {
            if let Err(e) = self.disconnect().await {
                warn!("Error disconnecting previous session: {}", e);
            }
        }

        let app_handle = self.app_handle.clone().ok_or("App handle not set")?;

        // Parse the JID
        let jid: jid::BareJid = jid_str
            .parse()
            .map_err(|e| format!("Invalid JID '{}': {}", jid_str, e))?;

        // Use the JID domain as the XMPP domain and allow a separate network host/port
        // for local testing through NTO_XMPP_HOST and NTO_XMPP_PORT.
        let stop_flag = Arc::new(AtomicBool::new(false));
        let domain = jid.domain().to_string();
        let host = std::env::var("NTO_XMPP_HOST").unwrap_or(domain);
        let port = xmpp_port();
        let allow_invalid_certs = xmpp_allow_invalid_certs();
        let client = {
            use configured_connector::ConfiguredTlsConnector;
            tokio_xmpp::Client::new_with_connector(
                jid,
                password,
                ConfiguredTlsConnector {
                    host,
                    port,
                    allow_invalid_certs,
                    stop: stop_flag.clone(),
                },
                tokio_xmpp::xmlstream::Timeouts::default(),
            )
        };

        // Create the command channel
        let (tx, rx) = mpsc::channel::<OutgoingCmd>(32);

        // Oneshot channel: event loop fires Ok(jid) on first Online, Err(msg) on first Disconnected
        let (auth_tx, auth_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

        // Clone state for the background task
        let status_arc = self.connection_status.clone();
        let own_jid = jid_str.clone();

        let pending_subs_arc = self.pending_subscriptions.clone();
        let pending_group_invites_arc = self.pending_group_invites.clone();
        let presence_cache_arc = self.presence_cache.clone();
        let current_show_arc = self.current_show.clone();

        // Spawn the background event loop
        let handle = tokio::spawn(xmpp_event_loop(
            client,
            rx,
            app_handle,
            XmppEventLoopState {
                status: status_arc,
                own_jid,
                pending_subscriptions: pending_subs_arc,
                pending_group_invites: pending_group_invites_arc,
                presence_cache: presence_cache_arc,
                current_show: current_show_arc,
            },
            Some(auth_tx),
        ));

        self.sender = Some(tx);
        self.task_handle = Some(handle);

        // Wait for auth result, with a timeout to guard against tokio-xmpp's infinite
        // reconnect loop on auth failures (NotAuthorized is retried forever inside
        // StanzaStream::new_c2s — see the "TODO: auth errors should probably be fatal"
        // comment in tokio-xmpp's stanzastream/mod.rs).
        let auth_result = tokio::time::timeout(std::time::Duration::from_secs(15), auth_rx).await;

        let outcome = match auth_result {
            Ok(Ok(Ok(()))) => {
                info!("XMPP authentication confirmed for JID: {}", jid_str);
                return Ok(());
            }
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(_)) => Err("Connection attempt failed unexpectedly".to_string()),
            Err(_elapsed) => Err("Invalid credentials or connection timed out".to_string()),
        };

        // Clean up on any failure path.
        // Set the stop flag first so the connector's pending() trap catches the next retry
        // attempt from tokio-xmpp's detached reconnector task (which we cannot cancel directly).
        stop_flag.store(true, Ordering::Relaxed);
        if let Some(sender) = self.sender.take() {
            sender.send(OutgoingCmd::Disconnect).await.ok();
        }
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        outcome
    }

    pub async fn send_nudge(&self, to_jid: String) -> Result<(), String> {
        if to_jid.contains("@conference.") {
            return Err("Nudge is not supported for group chats".to_string());
        }
        let status = self.connection_status.lock().await;
        if !status.connected {
            return Err("Not connected to XMPP server".to_string());
        }
        drop(status);
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::SendNudge { to: to_jid })
            .await
            .map_err(|e| format!("Failed to queue nudge: {e}"))
    }

    pub async fn send_message(&self, to_jid: String, body: String) -> Result<(), String> {
        info!("Queueing message send to {}", to_jid);
        {
            let status = self.connection_status.lock().await;
            if !status.connected {
                return Err("Not connected to XMPP server".to_string());
            }
        }
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::SendMessage { to: to_jid, body })
            .await
            .map_err(|e| format!("Failed to queue message: {}", e))
    }

    pub async fn create_group(
        &self,
        room_name: String,
        invitees: Vec<String>,
    ) -> Result<String, String> {
        let status = self.connection_status.lock().await;
        if !status.connected {
            return Err("Not connected to XMPP server".to_string());
        }
        let own_bare = status
            .jid
            .as_deref()
            .unwrap_or("")
            .split('/')
            .next()
            .unwrap_or("")
            .to_string();
        drop(status);
        if own_bare.is_empty() {
            return Err("Current JID is unavailable".to_string());
        }
        let name = room_name.trim();
        if name.is_empty() || name.len() > 80 {
            return Err("Group name must contain 1 to 80 characters".to_string());
        }
        let slug = name
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        if slug.is_empty() {
            return Err("Group name must contain a letter or number".to_string());
        }
        let room_id = format!("{slug}-{}", uuid::Uuid::new_v4());
        let room_jid = format!("{room_id}@conference.localhost");
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        let nickname = own_bare.split('@').next().unwrap_or("owner").to_string();
        sender
            .send(OutgoingCmd::JoinGroup {
                room_jid: room_jid.clone(),
                nickname: nickname.clone(),
            })
            .await
            .map_err(|e| format!("Failed to join created group: {e}"))?;
        sender
            .send(OutgoingCmd::SetGroupSubject {
                room_jid: room_jid.clone(),
                subject: name.to_string(),
            })
            .await
            .map_err(|e| format!("Failed to set group name: {e}"))?;
        for invitee in invitees {
            sender
                .send(OutgoingCmd::SendGroupInvite {
                    room_jid: room_jid.clone(),
                    room_name: name.to_string(),
                    inviter: own_bare.clone(),
                    invitee,
                })
                .await
                .map_err(|e| format!("Failed to queue group invite: {e}"))?;
        }
        Ok(room_jid)
    }

    pub async fn invite_to_group(
        &self,
        room_jid: String,
        room_name: String,
        invitees: Vec<String>,
    ) -> Result<(), String> {
        let status = self.connection_status.lock().await;
        if !status.connected {
            return Err("Not connected to XMPP server".to_string());
        }
        let inviter = status
            .jid
            .as_deref()
            .unwrap_or("")
            .split('/')
            .next()
            .unwrap_or("")
            .to_string();
        drop(status);
        if inviter.is_empty() {
            return Err("Current JID is unavailable".to_string());
        }
        if room_jid.parse::<jid::BareJid>().is_err() || !room_jid.contains("@conference.") {
            return Err("Invalid MUC room JID".to_string());
        }
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        for invitee in invitees {
            if invitee.parse::<jid::BareJid>().is_err() {
                return Err(format!("Invalid invitee JID: {invitee}"));
            }
            sender
                .send(OutgoingCmd::SendGroupInvite {
                    room_jid: room_jid.clone(),
                    room_name: room_name.clone(),
                    inviter: inviter.clone(),
                    invitee,
                })
                .await
                .map_err(|e| format!("Failed to queue group invite: {e}"))?;
        }
        Ok(())
    }

    pub async fn join_group(&self, room_jid: String) -> Result<(), String> {
        let status = self.connection_status.lock().await;
        if !status.connected {
            return Err("Not connected to XMPP server".to_string());
        }
        let own_bare = status
            .jid
            .as_deref()
            .unwrap_or("")
            .split('/')
            .next()
            .unwrap_or("")
            .to_string();
        drop(status);
        let nickname = own_bare.split('@').next().unwrap_or("member").to_string();
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::JoinGroup {
                room_jid: room_jid.clone(),
                nickname: nickname.clone(),
            })
            .await
            .map_err(|e| format!("Failed to join group: {e}"))?;
        self.pending_group_invites
            .lock()
            .await
            .retain(|invite| invite.room_jid != room_jid);
        Ok(())
    }

    pub async fn send_group_message(&self, room_jid: String, body: String) -> Result<(), String> {
        if body.trim().is_empty() || body.len() > 4096 {
            return Err("Message must contain 1 to 4096 bytes".to_string());
        }
        let status = self.connection_status.lock().await;
        if !status.connected {
            return Err("Not connected to XMPP server".to_string());
        }
        drop(status);
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::SendGroupMessage { room_jid, body })
            .await
            .map_err(|e| format!("Failed to queue group message: {e}"))
    }

    pub async fn leave_group(&self, room_jid: String) -> Result<(), String> {
        let status = self.connection_status.lock().await;
        let nickname = status
            .jid
            .as_deref()
            .unwrap_or("member")
            .split('/')
            .next()
            .unwrap_or("member")
            .split('@')
            .next()
            .unwrap_or("member")
            .to_string();
        drop(status);
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::LeaveGroup { room_jid, nickname })
            .await
            .map_err(|e| format!("Failed to queue group leave: {e}"))
    }

    pub async fn set_group_subject(&self, room_jid: String, subject: String) -> Result<(), String> {
        if subject.trim().is_empty() || subject.len() > 80 {
            return Err("Group name must contain 1 to 80 characters".to_string());
        }
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::SetGroupSubject { room_jid, subject })
            .await
            .map_err(|e| format!("Failed to queue group subject update: {e}"))
    }

    pub async fn remove_group_member(
        &self,
        room_jid: String,
        member_jid: String,
    ) -> Result<(), String> {
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::RemoveGroupMember {
                room_jid,
                member_jid,
            })
            .await
            .map_err(|e| format!("Failed to queue member removal: {e}"))
    }

    pub async fn drain_pending_group_invites(&self) -> Vec<XmppGroupInvite> {
        let mut invites = self.pending_group_invites.lock().await;
        invites.drain(..).collect()
    }

    pub async fn set_presence(
        &self,
        availability: Availability,
        status: Option<String>,
    ) -> Result<(), String> {
        info!("Queueing presence update: {:?}", availability);
        {
            let conn_status = self.connection_status.lock().await;
            if !conn_status.connected {
                return Err("Not connected to XMPP server".to_string());
            }
        }
        let show = match availability {
            Availability::Away => Some("away".to_string()),
            Availability::Busy => Some("dnd".to_string()),
            Availability::Online | Availability::Offline => None,
        };
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::SetPresence {
                show,
                status_text: status,
                unavailable: matches!(availability, Availability::Offline),
            })
            .await
            .map_err(|e| format!("Failed to queue presence: {}", e))
    }

    pub async fn add_contact(&self, jid: String) -> Result<(), String> {
        info!("Adding contact: {}", jid);
        {
            let status = self.connection_status.lock().await;
            if !status.connected {
                return Err("Not connected to XMPP server".to_string());
            }
        }
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::AddContact { jid })
            .await
            .map_err(|e| format!("Failed to queue add contact: {}", e))
    }

    pub async fn request_roster(&self) -> Result<(), String> {
        info!("Queueing roster request");
        {
            let status = self.connection_status.lock().await;
            if !status.connected {
                return Err("Not connected to XMPP server".to_string());
            }
        }
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::RequestRoster)
            .await
            .map_err(|e| format!("Failed to queue roster request: {}", e))
    }

    pub async fn fetch_history(&self) -> Result<(), String> {
        info!("Queueing MAM history request");
        {
            let status = self.connection_status.lock().await;
            if !status.connected {
                return Err("Not connected to XMPP server".to_string());
            }
        }
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::FetchHistory)
            .await
            .map_err(|e| format!("Failed to queue MAM history request: {}", e))
    }

    pub async fn fetch_group_history(&self, room_jid: String) -> Result<(), String> {
        info!("Queueing MUC-MAM history request for {}", room_jid);
        {
            let status = self.connection_status.lock().await;
            if !status.connected {
                return Err("Not connected to XMPP server".to_string());
            }
        }
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::FetchGroupHistory { room_jid })
            .await
            .map_err(|e| format!("Failed to queue MUC-MAM history request: {e}"))
    }

    pub async fn fetch_vcard(&self) -> Result<(), String> {
        info!("Queueing vCard fetch");
        {
            let status = self.connection_status.lock().await;
            if !status.connected {
                return Err("Not connected to XMPP server".to_string());
            }
        }
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::FetchVcard)
            .await
            .map_err(|e| format!("Failed to queue vCard fetch: {}", e))
    }

    pub async fn fetch_contact_vcard(&self, jid: String) -> Result<(), String> {
        info!("Queueing contact vCard fetch for {}", jid);
        {
            let status = self.connection_status.lock().await;
            if !status.connected {
                return Err("Not connected to XMPP server".to_string());
            }
        }
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::FetchContactVcard { jid })
            .await
            .map_err(|e| format!("Failed to queue contact vCard fetch: {}", e))
    }

    pub async fn get_presence_cache(&self) -> Vec<XmppPresence> {
        let cache = self.presence_cache.lock().await;
        cache
            .iter()
            .filter_map(|(bare_jid, resources)| {
                let aggregate = aggregate_presence(bare_jid, resources);
                (!matches!(aggregate.availability, Availability::Offline)).then_some(aggregate)
            })
            .collect()
    }

    pub async fn broadcast_presence(&self) -> Result<(), String> {
        {
            let status = self.connection_status.lock().await;
            if !status.connected {
                return Err("Not connected".to_string());
            }
        }
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::BroadcastPresence)
            .await
            .map_err(|e| format!("Failed to queue broadcast presence: {}", e))
    }

    pub async fn accept_subscription(&self, jid: String) -> Result<(), String> {
        info!("Accepting subscription from {}", jid);
        {
            let status = self.connection_status.lock().await;
            if !status.connected {
                return Err("Not connected to XMPP server".to_string());
            }
        }
        self.pending_subscriptions
            .lock()
            .await
            .retain(|j| j != &jid);
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::AcceptAndSubscribe { jid })
            .await
            .map_err(|e| format!("Failed to queue subscription accept: {}", e))
    }

    pub async fn deny_subscription(&self, jid: String) -> Result<(), String> {
        info!("Denying subscription from {}", jid);
        {
            let status = self.connection_status.lock().await;
            if !status.connected {
                return Err("Not connected to XMPP server".to_string());
            }
        }
        self.pending_subscriptions
            .lock()
            .await
            .retain(|j| j != &jid);
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::SendPresenceDirected {
                to: jid,
                pres_type: "unsubscribed".to_string(),
            })
            .await
            .map_err(|e| format!("Failed to queue subscription deny: {}", e))
    }

    pub async fn set_vcard(&self, nickname: String, desc: String) -> Result<(), String> {
        info!("Queueing vCard update: nickname='{}'", nickname);
        {
            let status = self.connection_status.lock().await;
            if !status.connected {
                return Err("Not connected to XMPP server".to_string());
            }
        }
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::SetVcard { nickname, desc })
            .await
            .map_err(|e| format!("Failed to queue vCard update: {}", e))
    }

    pub async fn disconnect(&mut self) -> Result<(), String> {
        info!("Disconnecting from XMPP server");

        if let Some(sender) = self.sender.take() {
            sender.send(OutgoingCmd::Disconnect).await.ok();
        }
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }

        {
            let mut status = self.connection_status.lock().await;
            status.connected = false;
            status.jid = None;
        }
        self.presence_cache.lock().await.clear();

        if let Some(ref app_handle) = self.app_handle {
            let status = ConnectionStatus {
                connected: false,
                jid: None,
                error: None,
            };
            app_handle
                .emit("xmpp_disconnected", &status)
                .map_err(|e| format!("Failed to emit disconnected event: {}", e))?;
        }

        info!("XMPP disconnection completed");
        Ok(())
    }

    pub async fn drain_pending_subscriptions(&self) -> Vec<String> {
        let mut subs = self.pending_subscriptions.lock().await;
        subs.drain(..).collect()
    }

    pub async fn is_connected(&self) -> bool {
        self.connection_status.lock().await.connected
    }

    pub async fn get_current_jid(&self) -> Option<String> {
        self.connection_status.lock().await.jid.clone()
    }
}

fn subscription_str(s: &Subscription) -> &'static str {
    match s {
        Subscription::Both => "both",
        Subscription::From => "from",
        Subscription::To => "to",
        Subscription::Remove => "remove",
        Subscription::None => "none",
    }
}

async fn xmpp_event_loop(
    mut client: tokio_xmpp::Client,
    mut rx: mpsc::Receiver<OutgoingCmd>,
    app_handle: tauri::AppHandle,
    state: XmppEventLoopState,
    mut auth_result: Option<tokio::sync::oneshot::Sender<Result<(), String>>>,
) {
    loop {
        tokio::select! {
            event = client.next() => {
                match event {
                    Some(Event::Online { bound_jid, .. }) => {
                        info!("XMPP online! Bound JID: {}", bound_jid);
                        let jid_string = bound_jid.to_string();
                        {
                            let mut s = state.status.lock().await;
                            s.connected = true;
                            s.jid = Some(jid_string.clone());
                            s.error = None;
                        }
                        // Signal auth success back to connect()
                        if let Some(tx) = auth_result.take() {
                            let _ = tx.send(Ok(()));
                        }
                        let conn_status = ConnectionStatus {
                            connected: true,
                            jid: Some(jid_string),
                            error: None,
                        };
                        if let Err(e) = app_handle.emit("xmpp_connected", &conn_status) {
                            error!("Failed to emit xmpp_connected: {}", e);
                        }
                        // Phase D: broadcast initial available presence
                        if let Err(e) = client.send_stanza(Stanza::Presence(Presence::available())).await {
                            error!("Failed to send initial presence: {}", e);
                        }
                        // Phase C: request roster from server
                        let roster_iq = XmppIq::from_get("roster-1", Roster { ver: None, items: vec![] });
                        if let Err(e) = client.send_stanza(Stanza::Iq(roster_iq)).await {
                            error!("Failed to send roster IQ: {}", e);
                        }
                    }
                    Some(Event::Stanza(Stanza::Message(msg))) => {
                        // Skip error stanzas (e.g. bounce from non-existent recipient)
                        if msg.type_ == MessageType::Error {
                            warn!("Skipping error message stanza from {:?}", msg.from);
                            continue;
                        }

                        if let Some(nudge) = msg.payloads.iter().find(|payload| {
                            payload.name() == "nudge" && payload.ns() == "urn:xmpp:nudge:0"
                        }) {
                            let _ = nudge;
                            let from_full = msg
                                .from
                                .as_ref()
                                .map(|jid| jid.to_string())
                                .unwrap_or_default();
                            let timestamp = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                                .to_string();
                            let payload = serde_json::json!({
                                "from": from_full,
                                "timestamp": timestamp,
                            });
                            if let Err(e) = app_handle.emit("xmpp_nudge_received", &payload) {
                                error!("Failed to emit xmpp_nudge_received: {}", e);
                            }
                            continue;
                        }

                        if let Some(muc_user) = msg.payloads.iter().find(|payload| {
                            payload.name() == "x"
                                && payload.ns() == "http://jabber.org/protocol/muc#user"
                        }) {
                            if let Some(invite) = muc_user.get_child(
                                "invite",
                                "http://jabber.org/protocol/muc#user",
                            ) {
                                let room_jid = invite
                                    .attr("to")
                                    .unwrap_or("")
                                    .to_string();
                                let inviter = invite
                                    .attr("from")
                                    .unwrap_or("")
                                    .to_string();
                                let room_name = msg
                                    .get_best_subject(vec!["en", ""])
                                    .map(|(_, subject)| subject.clone())
                                    .filter(|subject| !subject.is_empty())
                                    .unwrap_or_else(|| {
                                        room_jid
                                            .split('@')
                                            .next()
                                            .unwrap_or(&room_jid)
                                            .to_string()
                                    });
                                if !room_jid.is_empty() {
                                    let invite = XmppGroupInvite {
                                        room_jid,
                                        room_name,
                                        inviter,
                                    };
                                    let mut invites =
                                        state.pending_group_invites.lock().await;
                                    if !invites.iter().any(|item| {
                                        item.room_jid == invite.room_jid
                                            && item.inviter == invite.inviter
                                    }) {
                                        invites.push(invite.clone());
                                    }
                                    let payload = serde_json::json!({
                                        "room_jid": invite.room_jid,
                                        "room_name": invite.room_name,
                                        "inviter": invite.inviter,
                                    });
                                    if let Err(e) = app_handle.emit(
                                        "xmpp_group_invite_received",
                                        &payload,
                                    ) {
                                        error!("Failed to emit MUC invite: {}", e);
                                    }
                                }
                            }
                        }

                        // XEP-0313 MAM replay wraps the original stanza inside <forwarded>.
                        if let Some(forwarded) = msg.payloads.iter().find(|payload| {
                            payload.name() == "forwarded" && payload.ns() == "urn:xmpp:forward:0"
                        }) {
                            if let Some(archived) = forwarded.get_child("message", "jabber:client") {
                                let body = archived
                                    .get_child("body", "jabber:client")
                                    .map(|node| node.text())
                                    .filter(|text| !text.is_empty());
                                if let Some(body) = body {
                                    let from_full = archived.attr("from").unwrap_or("").to_string();
                                    let to_full = archived.attr("to").unwrap_or("").to_string();
                                    let is_groupchat = archived.attr("type") == Some("groupchat");
                                    let (peer_jid, is_self) = mam_peer_jid(
                                        &from_full,
                                        &to_full,
                                        &state.own_jid,
                                        is_groupchat,
                                    );
                                    if peer_jid.is_empty() {
                                        continue;
                                    }
                                    let timestamp = forwarded
                                        .get_child("delay", "urn:xmpp:delay")
                                        .and_then(|delay| delay.attr("stamp"))
                                        .unwrap_or("")
                                        .to_string();
                                    let timestamp = if timestamp.is_empty() {
                                        let seconds = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs();
                                        format!("{seconds}")
                                    } else {
                                        timestamp
                                    };
                                    let msg_id = archived
                                        .attr("id")
                                        .unwrap_or("")
                                        .to_string();
                                    let stanza_id = forwarded
                                        .get_child("stanza-id", "urn:xmpp:sid:0")
                                        .and_then(|stanza| stanza.attr("id"))
                                        .or_else(|| {
                                            archived
                                                .get_child("stanza-id", "urn:xmpp:sid:0")
                                                .and_then(|stanza| stanza.attr("id"))
                                        })
                                        .unwrap_or("")
                                        .to_string();
                                    let msg_id = if !stanza_id.is_empty() {
                                        stanza_id
                                    } else {
                                        msg_id
                                    };
                                    let group_room = if is_groupchat {
                                        muc_room_from_jids(&from_full, &to_full)
                                    } else {
                                        None
                                    };
                                    let author_nickname = if is_groupchat {
                                        from_full
                                            .split('/')
                                            .nth(1)
                                            .or_else(|| to_full.split('/').nth(1))
                                            .map(str::to_string)
                                            .or_else(|| {
                                                if is_self {
                                                    None
                                                } else {
                                                    Some(from_full
                                                        .split('@')
                                                        .next()
                                                        .unwrap_or(&from_full)
                                                        .to_string())
                                                }
                                            })
                                    } else {
                                        None
                                    };
                                    let xmpp_msg = XmppMessage {
                                        id: msg_id,
                                        from: from_full,
                                        to: to_full,
                                        body,
                                        timestamp,
                                        message_type: "chat".to_string(),
                                        is_self,
                                        group_room,
                                        author_nickname,
                                    };
                                    if let Err(e) =
                                        app_handle.emit("xmpp_message_received", &xmpp_msg)
                                    {
                                        error!("Failed to emit archived xmpp_message_received: {}", e);
                                    }
                                }
                            }
                            continue;
                        }

                        if let Some((_, body_text)) = msg.get_best_body(vec!["en", ""]) {
                            let from_jid = msg.from.as_ref()
                                .map(|j| j.to_string())
                                .unwrap_or_default();
                            if msg.type_ == MessageType::Groupchat {
                                info!("Received MUC message from {}", from_jid);
                            }
                            let to_jid = msg
                                .to
                                .as_ref()
                                .map(|jid| jid.to_string())
                                .unwrap_or_else(|| state.own_jid.clone());
                            let msg_id = msg.id.as_ref()
                                .map(|id| id.0.clone())
                                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                            let xmpp_msg = XmppMessage {
                                id: msg_id,
                                from: from_jid.clone(),
                                to: to_jid,
                                body: body_text.clone(),
                                timestamp: {
                                    let seconds = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs();
                                    format!("{seconds}")
                                },
                                message_type: "chat".to_string(),
                                is_self: if msg.type_ == MessageType::Groupchat {
                                    from_jid
                                        .split('/')
                                        .nth(1)
                                        .map(|nickname| {
                                            nickname
                                                == state
                                                    .own_jid
                                                    .split('@')
                                                    .next()
                                                    .unwrap_or("")
                                        })
                                        .unwrap_or(false)
                                } else {
                                    live_individual_is_self(&from_jid, &state.own_jid)
                                },
                                group_room: if msg.type_ == MessageType::Groupchat {
                                    Some(muc_room_jid(&from_jid))
                                } else {
                                    None
                                },
                                author_nickname: if msg.type_ == MessageType::Groupchat {
                                    from_jid
                                        .split('/')
                                        .nth(1)
                                        .map(str::to_string)
                                } else {
                                    None
                                },
                            };
                            if let Err(e) = app_handle.emit("xmpp_message_received", &xmpp_msg) {
                                error!("Failed to emit xmpp_message_received: {}", e);
                            }
                        }
                    }
                    Some(Event::Stanza(Stanza::Presence(pres))) => {
                        // Handle incoming subscription requests
                        if pres.type_ == PresenceType::Subscribe {
                            let from_jid = pres.from.map(|j| j.to_string()).unwrap_or_default();
                            if !from_jid.is_empty() {
                                // Buffer for later retrieval (handles race with frontend mount)
                                {
                                    let mut subs = state.pending_subscriptions.lock().await;
                                    if !subs.contains(&from_jid) {
                                        subs.push(from_jid.clone());
                                    }
                                }
                                let payload = serde_json::json!({ "from_jid": from_jid });
                                if let Err(e) = app_handle.emit("xmpp_subscription_request", &payload) {
                                    error!("Failed to emit xmpp_subscription_request: {}", e);
                                }
                            }
                            continue;
                        }
                        if let Some(full_jid) = pres.from.as_ref().map(|jid| jid.to_string()) {
                            if full_jid.contains("@conference.") {
                                let room_jid = full_jid
                                    .split('/')
                                    .next()
                                    .unwrap_or(&full_jid)
                                    .to_string();
                                let nickname = full_jid
                                    .split('/')
                                    .nth(1)
                                    .unwrap_or("")
                                    .to_string();
                                let available = !matches!(pres.type_, PresenceType::Unavailable);
                                let availability = if available {
                                    match &pres.show {
                                        Some(Show::Away) => Availability::Away,
                                        Some(Show::Dnd) => Availability::Busy,
                                        _ => Availability::Online,
                                    }
                                } else {
                                    Availability::Offline
                                };
                                let member_jid = pres.payloads.iter()
                                    .find(|payload| payload.name() == "x" && payload.ns() == "http://jabber.org/protocol/muc#user")
                                    .and_then(|payload| payload.get_child("item", "http://jabber.org/protocol/muc#user"))
                                    .and_then(|item| item.attr("jid"))
                                    .unwrap_or("")
                                    .to_string();
                                let affiliation = pres.payloads.iter()
                                    .find(|payload| payload.name() == "x" && payload.ns() == "http://jabber.org/protocol/muc#user")
                                    .and_then(|payload| payload.get_child("item", "http://jabber.org/protocol/muc#user"))
                                    .and_then(|item| item.attr("affiliation"))
                                    .unwrap_or("member")
                                    .to_string();
                                let role = pres.payloads.iter()
                                    .find(|payload| payload.name() == "x" && payload.ns() == "http://jabber.org/protocol/muc#user")
                                    .and_then(|payload| payload.get_child("item", "http://jabber.org/protocol/muc#user"))
                                    .and_then(|item| item.attr("role"))
                                    .unwrap_or("participant")
                                    .to_string();
                                let payload = serde_json::json!({
                                    "room_jid": room_jid,
                                    "nickname": nickname,
                                    "jid": member_jid,
                                    "availability": availability,
                                    "available": available,
                                    "affiliation": affiliation,
                                    "role": role,
                                });
                                if let Err(e) = app_handle.emit("xmpp_group_member_update", &payload) {
                                    error!("Failed to emit MUC member update: {}", e);
                                }
                            }
                        }
                        // Phase D: handle contact presence updates
                        let availability = match &pres.type_ {
                            PresenceType::None => Some(match &pres.show {
                                Some(Show::Away) => Availability::Away,
                                Some(Show::Dnd) => Availability::Busy,
                                _ => Availability::Online,
                            }),
                            PresenceType::Unavailable => Some(Availability::Offline),
                            _ => None,
                        };
                        if let Some(avail) = availability {
                            let jid_full = pres.from.map(|j| j.to_string()).unwrap_or_default();
                            if !jid_full.is_empty() {
                                if jid_full.contains("@conference.") {
                                    continue;
                                }
                                // Strip resource to get bare JID for cache key
                                let bare_jid = jid_full.split('/').next().unwrap_or(&jid_full).to_string();
                                // Skip own presence echoes from the server
                                let own_bare = state.own_jid.split('/').next().unwrap_or(&state.own_jid).to_string();
                                if bare_jid != own_bare {
                                    let status_text = pres
                                        .statuses
                                        .get(&Lang::new())
                                        .map(|status| status.to_string());
                                    let xmpp_pres = XmppPresence {
                                        jid: bare_jid.clone(),
                                        availability: avail,
                                        status: status_text,
                                    };
                                    // Write to cache so MainPage can replay on mount
                                    let xmpp_pres = {
                                        let mut cache = state.presence_cache.lock().await;
                                        update_presence_resource(
                                            &mut cache,
                                            &jid_full,
                                            xmpp_pres.availability.clone(),
                                            xmpp_pres.status.clone(),
                                        )
                                    };
                                    if let Err(e) = app_handle.emit("xmpp_presence_update", &xmpp_pres) {
                                        error!("Failed to emit xmpp_presence_update: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Some(Event::Stanza(Stanza::Iq(iq))) => {
                        match iq {
                            XmppIq::Result { payload: Some(elem), from: iq_from, .. } => {
                                if elem.name() == "vCard" && elem.ns() == "vcard-temp" {
                                    let nickname = elem
                                        .get_child("NICKNAME", "vcard-temp")
                                        .and_then(|n| n.texts().next())
                                        .map(|s| s.to_string())
                                        .unwrap_or_default();
                                    let flavour_text = elem
                                        .get_child("DESC", "vcard-temp")
                                        .and_then(|n| n.texts().next())
                                        .map(|s| s.to_string())
                                        .unwrap_or_default();
                                    // `from` is set for contact vCards, absent for own vCard
                                    let vcard_jid = iq_from
                                        .as_ref()
                                        .map(|j| j.to_string())
                                        .map(|j| j.split('/').next().unwrap_or(&j).to_string())
                                        .unwrap_or_default();
                                    let payload = serde_json::json!({
                                        "jid": vcard_jid,
                                        "nickname": nickname,
                                        "flavour_text": flavour_text,
                                    });
                                    info!("vCard received (jid='{}'): nickname='{}', desc='{}'", vcard_jid, nickname, flavour_text);
                                    if let Err(e) = app_handle.emit("xmpp_vcard_received", &payload) {
                                        error!("Failed to emit xmpp_vcard_received: {}", e);
                                    }
                                } else if let Ok(roster) = Roster::try_from(elem) {
                                    let contacts: Vec<serde_json::Value> = roster.items.iter().map(|item| {
                                        serde_json::json!({
                                            "jid": item.jid.to_string(),
                                            "name": item.name.as_deref().unwrap_or(""),
                                            "subscription": subscription_str(&item.subscription),
                                        })
                                    }).collect();
                                    info!("Roster received with {} contacts", contacts.len());
                                    if let Err(e) = app_handle.emit("xmpp_roster_received", &contacts) {
                                        error!("Failed to emit xmpp_roster_received: {}", e);
                                    }
                                    for item in &roster.items {
                                        if matches!(
                                            item.subscription,
                                            Subscription::Both | Subscription::From
                                        ) {
                                            let mut probe =
                                                Presence::new(PresenceType::Probe);
                                            probe.to = Some(jid::Jid::from(item.jid.clone()));
                                            if let Err(e) =
                                                client.send_stanza(Stanza::Presence(probe)).await
                                            {
                                                error!(
                                                    "Failed to probe presence for '{}': {}",
                                                    item.jid, e
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            // Roster push from server (subscription state changes, new contacts)
                            XmppIq::Set { id, payload, .. } => {
                                // Must acknowledge the push
                                let ack = XmppIq::Result {
                                    from: None,
                                    to: None,
                                    id,
                                    payload: None::<MinidomElement>,
                                };
                                if let Err(e) = client.send_stanza(Stanza::Iq(ack)).await {
                                    error!("Failed to ack roster push: {}", e);
                                }
                                if let Ok(roster) = Roster::try_from(payload) {
                                    for item in &roster.items {
                                        let push = serde_json::json!({
                                            "jid": item.jid.to_string(),
                                            "name": item.name.as_deref().unwrap_or(""),
                                            "subscription": subscription_str(&item.subscription),
                                        });
                                        if let Err(e) = app_handle.emit("xmpp_roster_push", &push) {
                                            error!("Failed to emit xmpp_roster_push: {}", e);
                                        }
                                        if matches!(
                                            item.subscription,
                                            Subscription::Both | Subscription::From
                                        ) {
                                            let mut probe = Presence::new(PresenceType::Probe);
                                            probe.to = Some(jid::Jid::from(item.jid.clone()));
                                            if let Err(e) =
                                                client.send_stanza(Stanza::Presence(probe)).await
                                            {
                                                error!(
                                                    "Failed to probe presence for '{}': {}",
                                                    item.jid, e
                                                );
                                            }
                                        } else if matches!(
                                            item.subscription,
                                            Subscription::Remove | Subscription::None
                                        ) {
                                            state
                                                .presence_cache
                                                .lock()
                                                .await
                                                .remove(&item.jid.to_string());
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    Some(Event::Disconnected(err)) => {
                        error!("XMPP disconnected: {}", err);
                        {
                            let mut s = state.status.lock().await;
                            s.connected = false;
                            s.jid = None;
                            s.error = Some(err.to_string());
                        }
                        state.presence_cache.lock().await.clear();
                        // Signal auth failure back to connect() if it's still waiting
                        if let Some(tx) = auth_result.take() {
                            let _ = tx.send(Err(err.to_string()));
                        }
                        let conn_status = ConnectionStatus {
                            connected: false,
                            jid: None,
                            error: Some(err.to_string()),
                        };
                        if let Err(e) = app_handle.emit("xmpp_disconnected", &conn_status) {
                            error!("Failed to emit xmpp_disconnected: {}", e);
                        }
                        break;
                    }
                    None => {
                        info!("XMPP stream ended");
                        break;
                    }
                }
            }
            cmd = rx.recv() => {
                match cmd {
                    Some(OutgoingCmd::SendMessage { to, body }) => {
                        match to.parse::<jid::BareJid>() {
                            Ok(to_jid) => {
                                let msg = XmppMsg::chat(jid::Jid::from(to_jid))
                                    .with_body(Default::default(), body);
                                if let Err(e) = client.send_stanza(Stanza::Message(msg)).await {
                                    error!("Failed to send message stanza: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Invalid recipient JID '{}': {}", to, e);
                            }
                        }
                    }
                    Some(OutgoingCmd::SendNudge { to }) => {
                        match to.parse::<jid::BareJid>() {
                            Ok(to_jid) => {
                                let mut msg = XmppMsg::chat(jid::Jid::from(to_jid));
                                msg.payloads.push(
                                    MinidomElement::builder("nudge", "urn:xmpp:nudge:0").build(),
                                );
                                if let Err(e) = client.send_stanza(Stanza::Message(msg)).await {
                                    error!("Failed to send nudge stanza: {}", e);
                                }
                            }
                            Err(e) => error!("Invalid nudge recipient '{}': {}", to, e),
                        }
                    }
                    Some(OutgoingCmd::SendGroupMessage { room_jid, body }) => {
                        match room_jid.parse::<jid::BareJid>() {
                            Ok(room) => {
                                let mut msg = XmppMsg::chat(jid::Jid::from(room));
                                msg.type_ = MessageType::Groupchat;
                                msg = msg.with_body(Default::default(), body);
                                info!("Sending MUC message to {}", room_jid);
                                if let Err(e) = client.send_stanza(Stanza::Message(msg)).await {
                                    error!("Failed to send group message stanza: {}", e);
                                }
                            }
                            Err(e) => error!("Invalid group room JID '{}': {}", room_jid, e),
                        }
                    }
                    Some(OutgoingCmd::JoinGroup { room_jid, nickname }) => {
                        let full_room = format!("{room_jid}/{nickname}");
                        match jid::Jid::new(&full_room) {
                            Ok(room) => {
                                info!("Joining MUC room {} as {}", room_jid, nickname);
                                let mut presence = Presence::available();
                                presence.to = Some(room);
                                let muc = MinidomElement::builder("x", "http://jabber.org/protocol/muc").build();
                                presence.payloads.push(muc);
                                if let Err(e) = client.send_stanza(Stanza::Presence(presence)).await {
                                    error!("Failed to join MUC room '{}': {}", room_jid, e);
                                }
                            }
                            Err(e) => error!("Invalid MUC room JID '{}': {}", room_jid, e),
                        }
                    }
                    Some(OutgoingCmd::LeaveGroup { room_jid, nickname }) => {
                        let full_room = format!("{room_jid}/{nickname}");
                        match jid::Jid::new(&full_room) {
                            Ok(room) => {
                                let mut presence = Presence::unavailable();
                                presence.to = Some(room);
                                if let Err(e) = client.send_stanza(Stanza::Presence(presence)).await {
                                    error!("Failed to leave MUC room '{}': {}", room_jid, e);
                                }
                            }
                            Err(e) => error!("Invalid MUC room JID '{}': {}", room_jid, e),
                        }
                    }
                    Some(OutgoingCmd::SendGroupInvite { room_jid, room_name, inviter, invitee }) => {
                        match (room_jid.parse::<jid::BareJid>(), invitee.parse::<jid::BareJid>()) {
                            (Ok(room), Ok(contact)) => {
                                let x = MinidomElement::builder(
                                    "x",
                                    "http://jabber.org/protocol/muc#user",
                                )
                                .append(
                                    MinidomElement::builder("invite", "http://jabber.org/protocol/muc#user")
                                        .attr(rxml::NcName::try_from("from").unwrap(), inviter)
                                        .attr(rxml::NcName::try_from("to").unwrap(), room.to_string())
                                        .build(),
                                )
                                .build();
                                let mut msg = XmppMsg::chat(jid::Jid::from(contact));
                                msg.payloads.push(x);
                                msg.subjects.insert(Lang::new(), room_name);
                                if let Err(e) = client.send_stanza(Stanza::Message(msg)).await {
                                    error!("Failed to send MUC invite to '{}': {}", invitee, e);
                                }
                            }
                            _ => error!("Invalid MUC invite room='{}' invitee='{}'", room_jid, invitee),
                        }
                    }
                    Some(OutgoingCmd::SetGroupSubject { room_jid, subject }) => {
                        match room_jid.parse::<jid::BareJid>() {
                            Ok(room) => {
                                let mut msg = XmppMsg::groupchat(jid::Jid::from(room));
                                msg.subjects.insert(Lang::new(), subject);
                                if let Err(e) = client.send_stanza(Stanza::Message(msg)).await {
                                    error!("Failed to set MUC room subject '{}': {}", room_jid, e);
                                }
                            }
                            Err(e) => error!("Invalid MUC room JID '{}': {}", room_jid, e),
                        }
                    }
                    Some(OutgoingCmd::RemoveGroupMember { room_jid, member_jid }) => {
                        match (room_jid.parse::<jid::BareJid>(), member_jid.parse::<jid::BareJid>()) {
                            (Ok(room), Ok(member)) => {
                                let item = MinidomElement::builder("item", "http://jabber.org/protocol/muc#admin")
                                    .attr(rxml::NcName::try_from("jid").unwrap(), member.to_string())
                                    .attr(rxml::NcName::try_from("affiliation").unwrap(), "none")
                                    .attr(rxml::NcName::try_from("role").unwrap(), "none")
                                    .build();
                                let query = MinidomElement::builder("query", "http://jabber.org/protocol/muc#admin")
                                    .append(item)
                                    .build();
                                let iq = XmppIq::Set {
                                    from: None,
                                    to: Some(jid::Jid::from(room)),
                                    id: format!("muc-admin-{}", uuid::Uuid::new_v4()),
                                    payload: query,
                                };
                                if let Err(e) = client.send_stanza(Stanza::Iq(iq)).await {
                                    error!("Failed to remove MUC member '{}': {}", member_jid, e);
                                }
                            }
                            _ => error!("Invalid MUC removal room='{}' member='{}'", room_jid, member_jid),
                        }
                    }
                    Some(OutgoingCmd::SetPresence {
                        show,
                        status_text,
                        unavailable,
                    }) => {
                        // Remember the current show value for BroadcastPresence
                        *state.current_show.lock().await = show.clone();
                        if unavailable {
                            let presence = Presence::unavailable();
                            if let Err(e) = client.send_stanza(Stanza::Presence(presence)).await {
                                error!("Failed to send unavailable presence stanza: {}", e);
                            }
                            continue;
                        }
                        let mut presence = Presence::available();
                        if let Some(show_str) = show {
                            presence = match show_str.as_str() {
                                "away" => presence.with_show(Show::Away),
                                "dnd" => presence.with_show(Show::Dnd),
                                "xa" => presence.with_show(Show::Xa),
                                "chat" => presence.with_show(Show::Chat),
                                _ => presence,
                            };
                        }
                        if let Some(text) = status_text {
                            presence.set_status(Lang::default(), text);
                        }
                        if let Err(e) = client.send_stanza(Stanza::Presence(presence)).await {
                            error!("Failed to send presence stanza: {}", e);
                        }
                    }
                    Some(OutgoingCmd::SetVcard { nickname, desc }) => {
                        let vcard_ns = "vcard-temp";
                        let mut vcard_builder = MinidomElement::builder("vCard", vcard_ns)
                            .append(
                                MinidomElement::builder("NICKNAME", vcard_ns)
                                    .append(minidom::Node::Text(nickname))
                                    .build()
                            );
                        if !desc.is_empty() {
                            vcard_builder = vcard_builder.append(
                                MinidomElement::builder("DESC", vcard_ns)
                                    .append(minidom::Node::Text(desc))
                                    .build()
                            );
                        }
                        let vcard_el = vcard_builder.build();
                        let iq = XmppIq::Set {
                            from: None,
                            to: None,
                            id: uuid::Uuid::new_v4().to_string(),
                            payload: vcard_el,
                        };
                        if let Err(e) = client.send_stanza(Stanza::Iq(iq)).await {
                            error!("Failed to send vCard set IQ: {}", e);
                        }
                    }
                    Some(OutgoingCmd::RequestRoster) => {
                        let roster_iq = XmppIq::from_get(
                            "roster-refresh",
                            Roster { ver: None, items: vec![] },
                        );
                        if let Err(e) = client.send_stanza(Stanza::Iq(roster_iq)).await {
                            error!("Failed to send roster IQ: {}", e);
                        }
                    }
                    Some(OutgoingCmd::FetchHistory) => {
                        let mam_id = format!("mam-{}", uuid::Uuid::new_v4());
                        let query_xml = format!(
                            "<query xmlns=\"urn:xmpp:mam:2\" queryid=\"{mam_id}\">\
                             <x xmlns=\"jabber:x:data\" type=\"submit\">\
                             <field var=\"FORM_TYPE\" type=\"hidden\">\
                             <value>urn:xmpp:mam:2</value>\
                             </field>\
                             </x>\
                             <set xmlns=\"http://jabber.org/protocol/rsm\"><max>200</max></set>\
                             </query>"
                        );
                        let query = match query_xml.parse::<MinidomElement>() {
                            Ok(query) => query,
                            Err(e) => {
                                error!("Failed to build MAM history query: {e}");
                                continue;
                            }
                        };
                        let iq = XmppIq::Set {
                            from: None,
                            to: None,
                            id: mam_id,
                            payload: query,
                        };
                        if let Err(e) = client.send_stanza(Stanza::Iq(iq)).await {
                            error!("Failed to send MAM history IQ: {}", e);
                        }
                    }
                    Some(OutgoingCmd::FetchGroupHistory { room_jid }) => {
                        match room_jid.parse::<jid::BareJid>() {
                            Ok(room) => {
                                let mam_id = format!("muc-mam-{}", uuid::Uuid::new_v4());
                                let query_xml = format!(
                                    "<query xmlns=\"urn:xmpp:mam:2\" queryid=\"{mam_id}\">\
                                     <x xmlns=\"jabber:x:data\" type=\"submit\">\
                                     <field var=\"FORM_TYPE\" type=\"hidden\">\
                                     <value>urn:xmpp:mam:2</value>\
                                     </field>\
                                     </x>\
                                     <set xmlns=\"http://jabber.org/protocol/rsm\"><max>500</max></set>\
                                     </query>"
                                );
                                match query_xml.parse::<MinidomElement>() {
                                    Ok(query) => {
                                        let iq = XmppIq::Set {
                                            from: None,
                                            to: Some(jid::Jid::from(room)),
                                            id: mam_id,
                                            payload: query,
                                        };
                                        if let Err(e) = client.send_stanza(Stanza::Iq(iq)).await {
                                            error!("Failed to send MUC-MAM history IQ: {}", e);
                                        }
                                    }
                                    Err(e) => error!("Failed to build MUC-MAM history query: {}", e),
                                }
                            }
                            Err(e) => error!("Invalid MUC room JID '{}': {}", room_jid, e),
                        }
                    }
                    Some(OutgoingCmd::FetchVcard) => {
                        let vcard_ns = "vcard-temp";
                        let vcard_el = MinidomElement::builder("vCard", vcard_ns).build();
                        let iq = XmppIq::Get {
                            from: None,
                            to: None,
                            id: uuid::Uuid::new_v4().to_string(),
                            payload: vcard_el,
                        };
                        if let Err(e) = client.send_stanza(Stanza::Iq(iq)).await {
                            error!("Failed to send vCard fetch IQ: {}", e);
                        }
                    }
                    Some(OutgoingCmd::FetchContactVcard { jid }) => {
                        match jid.parse::<jid::BareJid>() {
                            Ok(contact_jid) => {
                                let vcard_ns = "vcard-temp";
                                let vcard_el = MinidomElement::builder("vCard", vcard_ns).build();
                                let iq = XmppIq::Get {
                                    from: None,
                                    to: Some(jid::Jid::from(contact_jid)),
                                    id: uuid::Uuid::new_v4().to_string(),
                                    payload: vcard_el,
                                };
                                if let Err(e) = client.send_stanza(Stanza::Iq(iq)).await {
                                    error!("Failed to send contact vCard fetch IQ: {}", e);
                                }
                            }
                            Err(e) => error!("Invalid JID '{}' for contact vCard fetch: {}", jid, e),
                        }
                    }
                    Some(OutgoingCmd::BroadcastPresence) => {
                        let show = state.current_show.lock().await.clone();
                        let mut presence = Presence::available();
                        if let Some(show_str) = show {
                            presence = match show_str.as_str() {
                                "away" => presence.with_show(Show::Away),
                                "dnd" => presence.with_show(Show::Dnd),
                                _ => presence,
                            };
                        }
                        if let Err(e) = client.send_stanza(Stanza::Presence(presence)).await {
                            error!("Failed to broadcast presence: {}", e);
                        }
                    }
                    Some(OutgoingCmd::AcceptAndSubscribe { jid }) => {
                        match jid.parse::<jid::BareJid>() {
                            Ok(contact_jid) => {
                                // 1. Tell them we accepted their subscription
                                let mut subscribed_pres = Presence::new(PresenceType::Subscribed);
                                subscribed_pres.to = Some(jid::Jid::from(contact_jid.clone()));
                                if let Err(e) = client.send_stanza(Stanza::Presence(subscribed_pres)).await {
                                    error!("Failed to send subscribed presence: {}", e);
                                }
                                // 2. Add them to our roster
                                let item = RosterItem {
                                    jid: contact_jid.clone(),
                                    name: None,
                                    subscription: Subscription::None,
                                    ask: Ask::None,
                                    groups: vec![],
                                    approved: None,
                                };
                                let roster_set = Roster { ver: None, items: vec![item] };
                                let set_iq = XmppIq::Set {
                                    from: None,
                                    to: None,
                                    id: uuid::Uuid::new_v4().to_string(),
                                    payload: roster_set.into(),
                                };
                                if let Err(e) = client.send_stanza(Stanza::Iq(set_iq)).await {
                                    error!("Failed to send roster set on accept: {}", e);
                                }
                                // 3. Subscribe back (so we see their presence too)
                                let mut sub_pres = Presence::new(PresenceType::Subscribe);
                                sub_pres.to = Some(jid::Jid::from(contact_jid));
                                if let Err(e) = client.send_stanza(Stanza::Presence(sub_pres)).await {
                                    error!("Failed to send subscribe presence on accept: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Invalid JID '{}' for AcceptAndSubscribe: {}", jid, e);
                            }
                        }
                    }
                    Some(OutgoingCmd::AddContact { jid }) => {
                        match jid.parse::<jid::BareJid>() {
                            Ok(contact_jid) => {
                                let item = RosterItem {
                                    jid: contact_jid.clone(),
                                    name: None,
                                    subscription: Subscription::None,
                                    ask: Ask::None,
                                    groups: vec![],
                                    approved: None,
                                };
                                let roster_set = Roster { ver: None, items: vec![item] };
                                let set_iq = XmppIq::Set {
                                    from: None,
                                    to: None,
                                    id: uuid::Uuid::new_v4().to_string(),
                                    payload: roster_set.into(),
                                };
                                if let Err(e) = client.send_stanza(Stanza::Iq(set_iq)).await {
                                    error!("Failed to send roster set IQ: {}", e);
                                }
                                // Send subscribe presence to contact
                                let mut sub_pres = Presence::new(PresenceType::Subscribe);
                                sub_pres.to = Some(jid::Jid::from(contact_jid));
                                if let Err(e) = client.send_stanza(Stanza::Presence(sub_pres)).await {
                                    error!("Failed to send subscribe presence: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Invalid contact JID '{}': {}", jid, e);
                            }
                        }
                    }
                    Some(OutgoingCmd::SendPresenceDirected { to, pres_type }) => {
                        let ptype = match pres_type.as_str() {
                            "subscribed" => PresenceType::Subscribed,
                            "unsubscribed" => PresenceType::Unsubscribed,
                            "subscribe" => PresenceType::Subscribe,
                            _ => PresenceType::None,
                        };
                        match to.parse::<jid::BareJid>() {
                            Ok(target_jid) => {
                                let mut pres = Presence::new(ptype);
                                pres.to = Some(jid::Jid::from(target_jid));
                                if let Err(e) = client.send_stanza(Stanza::Presence(pres)).await {
                                    error!("Failed to send directed presence: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Invalid JID '{}' for directed presence: {}", to, e);
                            }
                        }
                    }
                    Some(OutgoingCmd::Disconnect) | None => {
                        info!("Disconnect command received, closing XMPP stream");
                        if let Err(e) = client.send_end().await {
                            warn!("Error during XMPP stream shutdown: {}", e);
                        }
                        break;
                    }
                }
            }
        }
    }
    info!("XMPP event loop terminated");
}

#[cfg(test)]
mod presence_tests {
    use super::*;

    #[test]
    fn aggregate_busy_over_online_resources() {
        let mut cache = PresenceCache::new();
        update_presence_resource(
            &mut cache,
            "friend@localhost/phone",
            Availability::Online,
            None,
        );
        let aggregate = update_presence_resource(
            &mut cache,
            "friend@localhost/desktop",
            Availability::Busy,
            Some("In a call".to_string()),
        );

        assert_eq!(aggregate.jid, "friend@localhost");
        assert!(matches!(aggregate.availability, Availability::Busy));
        assert_eq!(aggregate.status.as_deref(), Some("In a call"));
    }

    #[test]
    fn unavailable_resource_does_not_hide_another_resource() {
        let mut cache = PresenceCache::new();
        update_presence_resource(
            &mut cache,
            "friend@localhost/phone",
            Availability::Away,
            None,
        );
        update_presence_resource(
            &mut cache,
            "friend@localhost/desktop",
            Availability::Online,
            None,
        );
        let aggregate = update_presence_resource(
            &mut cache,
            "friend@localhost/phone",
            Availability::Offline,
            None,
        );

        assert_eq!(aggregate.jid, "friend@localhost");
        assert!(matches!(aggregate.availability, Availability::Online));
    }

    #[test]
    fn last_unavailable_resource_removes_contact_from_cache() {
        let mut cache = PresenceCache::new();
        update_presence_resource(
            &mut cache,
            "friend@localhost/desktop",
            Availability::Online,
            None,
        );
        let aggregate = update_presence_resource(
            &mut cache,
            "friend@localhost/desktop",
            Availability::Offline,
            None,
        );

        assert!(matches!(aggregate.availability, Availability::Offline));
        assert!(!cache.contains_key("friend@localhost"));
    }

    #[test]
    fn individual_outgoing_echo_is_self_by_bare_jid() {
        assert!(live_individual_is_self(
            "friend@localhost",
            "friend@localhost/mobile"
        ));
        assert!(live_individual_is_self(
            "pedro@localhost/resource",
            "pedro@localhost"
        ));
        assert!(!live_individual_is_self(
            "friend@localhost/resource",
            "pedro@localhost"
        ));
    }

    #[test]
    fn muc_message_routes_to_bare_room_jid() {
        assert_eq!(
            muc_room_jid("room@conference.localhost/pedro"),
            "room@conference.localhost"
        );
        assert_eq!(bare_xmpp_jid("friend@localhost/mobile"), "friend@localhost");
    }

    #[test]
    fn mam_routes_individual_and_muc_messages() {
        let (peer, is_self) = mam_peer_jid(
            "pedro@localhost/mobile",
            "friend@localhost",
            "pedro@localhost",
            false,
        );
        assert_eq!(peer, "friend@localhost");
        assert!(is_self);

        let (peer, is_self) = mam_peer_jid(
            "room@conference.localhost/pedro",
            "pedro@localhost",
            "pedro@localhost",
            true,
        );
        assert_eq!(peer, "room@conference.localhost");
        assert!(is_self);
    }
}
