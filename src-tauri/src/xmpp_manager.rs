use futures::StreamExt;
use log::{error, info, warn};
use minidom::Element as MinidomElement;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::Manager;
use tokio::sync::{mpsc, Mutex};
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct XmppPresence {
    pub jid: String,
    pub availability: Availability,
    pub status: Option<String>,
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
    SetPresence {
        show: Option<String>,
        status_text: Option<String>,
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

// Debug-only connector that skips TLS certificate validation.
// Allows the app to connect to a local dev server with a self-signed cert.
#[cfg(debug_assertions)]
mod insecure_connector {
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
    pub struct InsecureTlsConnector {
        pub host: String,
        pub port: u16,
        /// Set to true after an auth failure to silently halt tokio-xmpp's internal
        /// exponential-backoff retry loop (which runs in a detached task we can't cancel).
        pub stop: Arc<AtomicBool>,
    }

    impl ServerConnector for InsecureTlsConnector {
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

            // TLS with cert validation disabled — debug builds only!
            let native_connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .map_err(|e| {
                    XmppError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    ))
                })?;
            let tokio_connector = tokio_native_tls::TlsConnector::from(native_connector);
            let tls_stream = tokio_connector
                .connect(jid.domain().as_str(), tcp)
                .await
                .map_err(|e| {
                    XmppError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    ))
                })?;

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

pub struct XmppManager {
    sender: Option<mpsc::Sender<OutgoingCmd>>,
    task_handle: Option<tokio::task::JoinHandle<()>>,
    connection_status: Arc<Mutex<ConnectionStatus>>,
    app_handle: Option<tauri::AppHandle>,
    pending_subscriptions: Arc<Mutex<Vec<String>>>,
    /// Cache of last-known presence per bare JID. Populated by the event loop,
    /// read by `xmpp_get_presence_cache` so MainPage can replay on mount.
    presence_cache: Arc<Mutex<HashMap<String, XmppPresence>>>,
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
            presence_cache: Arc::new(Mutex::new(HashMap::new())),
            current_show: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_app_handle(&mut self, app_handle: tauri::AppHandle) {
        self.app_handle = Some(app_handle);
    }

    pub async fn connect(&mut self, jid_str: String, password: String) -> Result<(), String> {
        info!("Attempting XMPP connection for JID: {}", jid_str);

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

        // Create the XMPP client — in debug, skip TLS cert validation for local self-signed certs
        #[cfg(debug_assertions)]
        let stop_flag = Arc::new(AtomicBool::new(false));
        #[cfg(debug_assertions)]
        let client = {
            use insecure_connector::InsecureTlsConnector;
            let domain = jid.domain().to_string();
            tokio_xmpp::Client::new_with_connector(
                jid,
                password,
                InsecureTlsConnector {
                    host: domain,
                    port: 5223,
                    stop: stop_flag.clone(),
                },
                tokio_xmpp::xmlstream::Timeouts::default(),
            )
        };
        #[cfg(not(debug_assertions))]
        let client = tokio_xmpp::Client::new_direct_tls(jid, password);

        // Create the command channel
        let (tx, rx) = mpsc::channel::<OutgoingCmd>(32);

        // Oneshot channel: event loop fires Ok(jid) on first Online, Err(msg) on first Disconnected
        let (auth_tx, auth_rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

        // Clone state for the background task
        let status_arc = self.connection_status.clone();
        let own_jid = jid_str.clone();

        let pending_subs_arc = self.pending_subscriptions.clone();
        let presence_cache_arc = self.presence_cache.clone();
        let current_show_arc = self.current_show.clone();

        // Spawn the background event loop
        let handle = tokio::spawn(xmpp_event_loop(
            client,
            rx,
            app_handle,
            status_arc,
            own_jid,
            Some(auth_tx),
            pending_subs_arc,
            presence_cache_arc,
            current_show_arc,
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
        #[cfg(debug_assertions)]
        stop_flag.store(true, Ordering::Relaxed);
        if let Some(sender) = self.sender.take() {
            sender.send(OutgoingCmd::Disconnect).await.ok();
        }
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        outcome
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
        self.presence_cache.lock().await.values().cloned().collect()
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

        if let Some(ref app_handle) = self.app_handle {
            let status = ConnectionStatus {
                connected: false,
                jid: None,
                error: None,
            };
            app_handle
                .emit_all("xmpp_disconnected", &status)
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

    pub async fn get_connection_status(&self) -> ConnectionStatus {
        self.connection_status.lock().await.clone()
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
    status: Arc<Mutex<ConnectionStatus>>,
    own_jid: String,
    mut auth_result: Option<tokio::sync::oneshot::Sender<Result<(), String>>>,
    pending_subscriptions: Arc<Mutex<Vec<String>>>,
    presence_cache: Arc<Mutex<HashMap<String, XmppPresence>>>,
    current_show: Arc<Mutex<Option<String>>>,
) {
    loop {
        tokio::select! {
            event = client.next() => {
                match event {
                    Some(Event::Online { bound_jid, .. }) => {
                        info!("XMPP online! Bound JID: {}", bound_jid);
                        let jid_string = bound_jid.to_string();
                        {
                            let mut s = status.lock().await;
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
                        if let Err(e) = app_handle.emit_all("xmpp_connected", &conn_status) {
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
                        if let Some((_, body_text)) = msg.get_best_body(vec!["en", ""]) {
                            let from_jid = msg.from.as_ref()
                                .map(|j| j.to_string())
                                .unwrap_or_default();
                            let msg_id = msg.id.as_ref()
                                .map(|id| id.0.clone())
                                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                            let xmpp_msg = XmppMessage {
                                id: msg_id,
                                from: from_jid,
                                to: own_jid.clone(),
                                body: body_text.clone(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                message_type: "chat".to_string(),
                            };
                            if let Err(e) = app_handle.emit_all("xmpp_message_received", &xmpp_msg) {
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
                                    let mut subs = pending_subscriptions.lock().await;
                                    if !subs.contains(&from_jid) {
                                        subs.push(from_jid.clone());
                                    }
                                }
                                let payload = serde_json::json!({ "from_jid": from_jid });
                                if let Err(e) = app_handle.emit_all("xmpp_subscription_request", &payload) {
                                    error!("Failed to emit xmpp_subscription_request: {}", e);
                                }
                            }
                            continue;
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
                                // Strip resource to get bare JID for cache key
                                let bare_jid = jid_full.split('/').next().unwrap_or(&jid_full).to_string();
                                // Skip own presence echoes from the server
                                let own_bare = own_jid.split('/').next().unwrap_or(&own_jid).to_string();
                                if bare_jid != own_bare {
                                    let xmpp_pres = XmppPresence {
                                        jid: bare_jid.clone(),
                                        availability: avail,
                                        status: None,
                                    };
                                    // Write to cache so MainPage can replay on mount
                                    presence_cache.lock().await.insert(bare_jid, xmpp_pres.clone());
                                    if let Err(e) = app_handle.emit_all("xmpp_presence_update", &xmpp_pres) {
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
                                    if let Err(e) = app_handle.emit_all("xmpp_vcard_received", &payload) {
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
                                    if let Err(e) = app_handle.emit_all("xmpp_roster_received", &contacts) {
                                        error!("Failed to emit xmpp_roster_received: {}", e);
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
                                        if let Err(e) = app_handle.emit_all("xmpp_roster_push", &push) {
                                            error!("Failed to emit xmpp_roster_push: {}", e);
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
                            let mut s = status.lock().await;
                            s.connected = false;
                            s.jid = None;
                            s.error = Some(err.to_string());
                        }
                        // Signal auth failure back to connect() if it's still waiting
                        if let Some(tx) = auth_result.take() {
                            let _ = tx.send(Err(err.to_string()));
                        }
                        let conn_status = ConnectionStatus {
                            connected: false,
                            jid: None,
                            error: Some(err.to_string()),
                        };
                        if let Err(e) = app_handle.emit_all("xmpp_disconnected", &conn_status) {
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
                                let msg = XmppMsg::new(jid::Jid::from(to_jid)).with_body(Default::default(), body);
                                if let Err(e) = client.send_stanza(Stanza::Message(msg)).await {
                                    error!("Failed to send message stanza: {}", e);
                                }
                            }
                            Err(e) => {
                                error!("Invalid recipient JID '{}': {}", to, e);
                            }
                        }
                    }
                    Some(OutgoingCmd::SetPresence { show, status_text }) => {
                        // Remember the current show value for BroadcastPresence
                        *current_show.lock().await = show.clone();
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
                        let show = current_show.lock().await.clone();
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
