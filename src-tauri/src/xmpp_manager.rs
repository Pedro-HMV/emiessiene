use futures::StreamExt;
use log::{error, info, warn};
use minidom::Element as MinidomElement;
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::Manager;
use tokio::sync::{mpsc, Mutex};
use tokio_xmpp::parsers::iq::Iq as XmppIq;
use tokio_xmpp::parsers::message::{Lang, Message as XmppMsg};
use tokio_xmpp::parsers::presence::{Presence, Show, Type as PresenceType};
use tokio_xmpp::parsers::roster::Roster;
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
    SetVcardNickname(String),
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

        // Spawn the background event loop
        let handle = tokio::spawn(xmpp_event_loop(
            client,
            rx,
            app_handle,
            status_arc,
            own_jid,
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
        // Phase C will implement roster IQ; for now emit the event to keep compatibility
        if let Some(ref app_handle) = self.app_handle {
            app_handle
                .emit_all("xmpp_contact_added", &jid)
                .map_err(|e| format!("Failed to emit contact added event: {}", e))?;
        }
        Ok(())
    }

    pub async fn set_vcard_nickname(&self, nickname: String) -> Result<(), String> {
        info!("Queueing vCard nickname update: {}", nickname);
        {
            let status = self.connection_status.lock().await;
            if !status.connected {
                return Err("Not connected to XMPP server".to_string());
            }
        }
        let sender = self.sender.as_ref().ok_or("Not connected")?;
        sender
            .send(OutgoingCmd::SetVcardNickname(nickname))
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

async fn xmpp_event_loop(
    mut client: tokio_xmpp::Client,
    mut rx: mpsc::Receiver<OutgoingCmd>,
    app_handle: tauri::AppHandle,
    status: Arc<Mutex<ConnectionStatus>>,
    own_jid: String,
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
                            let jid_str = pres.from.map(|j| j.to_string()).unwrap_or_default();
                            if !jid_str.is_empty() {
                                let xmpp_pres = XmppPresence {
                                    jid: jid_str,
                                    availability: avail,
                                    status: None,
                                };
                                if let Err(e) = app_handle.emit_all("xmpp_presence_update", &xmpp_pres) {
                                    error!("Failed to emit xmpp_presence_update: {}", e);
                                }
                            }
                        }
                    }
                    Some(Event::Stanza(Stanza::Iq(iq))) => {
                        // Phase C: handle roster IQ result
                        if let XmppIq::Result { payload: Some(elem), .. } = iq {
                            match Roster::try_from(elem) {
                                Ok(roster) => {
                                    let contacts: Vec<serde_json::Value> = roster.items.iter().map(|item| {
                                        serde_json::json!({
                                            "jid": item.jid.to_string(),
                                            "name": item.name.as_deref().unwrap_or(""),
                                            "subscription": format!("{:?}", item.subscription),
                                        })
                                    }).collect();
                                    info!("Roster received with {} contacts", contacts.len());
                                    if let Err(e) = app_handle.emit_all("xmpp_roster_received", &contacts) {
                                        error!("Failed to emit xmpp_roster_received: {}", e);
                                    }
                                }
                                Err(e) => {
                                    warn!("Failed to parse roster IQ response: {}", e);
                                }
                            }
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
                    Some(OutgoingCmd::SetVcardNickname(nickname)) => {
                        let vcard_ns = "vcard-temp";
                        let vcard_el = MinidomElement::builder("vCard", vcard_ns)
                            .append(
                                MinidomElement::builder("NICKNAME", vcard_ns)
                                    .append(minidom::Node::Text(nickname))
                                    .build()
                            )
                            .build();
                        let iq = XmppIq::Set {
                            from: None,
                            to: None,
                            id: uuid::Uuid::new_v4().to_string(),
                            payload: vcard_el,
                        };
                        if let Err(e) = client.send_stanza(Stanza::Iq(iq)).await {
                            error!("Failed to send vCard nickname IQ: {}", e);
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
