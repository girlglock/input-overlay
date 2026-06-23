use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, watch, RwLock};
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use tauri::Emitter;

use crate::services::config::Config;
use crate::services::consts::{mouse_button_name, mouse_scroll_name, vk_to_key_name};

#[derive(Debug, Clone)]
pub enum InputEvent {
    KeyPress {
        rawcode: u16,
        timestamp: u64,
    },
    KeyRelease {
        rawcode: u16,
        timestamp: u64,
    },
    MouseButton {
        button: u8,
        pressed: bool,
        timestamp: u64,
    },
    MouseScroll {
        rotation: i8,
        timestamp: u64,
    },
    MouseMove {
        dx: i32,
        dy: i32,
        timestamp: u64,
    },
    AnalogDepth {
        rawcode: u16,
        depth: f32,
        timestamp: u64,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ServerStatus {
    pub running: bool,
    pub bind_error: Option<String>,
    pub client_count: usize,
    pub clients: Vec<String>,
    pub host: String,
    pub port: u16,
}

pub struct WsState {
    pub rebind_tx: watch::Sender<()>,
    pub input_tx: mpsc::UnboundedSender<InputEvent>,
}

fn is_allowed(event: &InputEvent, cfg: &Config) -> bool {
    if cfg.key_whitelist.is_empty() {
        return true;
    }
    match event {
        InputEvent::MouseMove { .. } => cfg.send_mouse_move,
        InputEvent::MouseScroll { rotation, .. } => {
            if cfg.key_whitelist.iter().any(|k| k == "mouse_wheel") {
                return true;
            }
            mouse_scroll_name(*rotation).is_some_and(|n| cfg.key_whitelist.contains(&n.to_string()))
        }
        InputEvent::MouseButton { button, .. } => {
            mouse_button_name(*button).is_some_and(|n| cfg.key_whitelist.contains(&n.to_string()))
        }
        InputEvent::KeyPress { rawcode, .. } | InputEvent::KeyRelease { rawcode, .. } => {
            vk_to_key_name(*rawcode).is_some_and(|n| cfg.key_whitelist.contains(&n.to_string()))
        }
        InputEvent::AnalogDepth { rawcode, .. } => {
            vk_to_key_name(*rawcode).is_some_and(|n| cfg.key_whitelist.contains(&n.to_string()))
        }
    }
}

fn event_to_json(event: &InputEvent) -> Option<String> {
    match event {
        InputEvent::KeyPress { rawcode, timestamp } => Some(format!(
            r#"{{"event_type":"key_pressed","rawcode":{rawcode},"timestamp":{timestamp}}}"#
        )),
        InputEvent::KeyRelease { rawcode, timestamp } => Some(format!(
            r#"{{"event_type":"key_released","rawcode":{rawcode},"timestamp":{timestamp}}}"#
        )),
        InputEvent::MouseButton {
            button,
            pressed,
            timestamp,
        } => {
            let event_type = if *pressed {
                "mouse_pressed"
            } else {
                "mouse_released"
            };
            Some(format!(
                r#"{{"event_type":"{event_type}","button":{button},"timestamp":{timestamp}}}"#
            ))
        }
        InputEvent::MouseScroll {
            rotation,
            timestamp,
        } => Some(format!(
            r#"{{"event_type":"mouse_wheel","rotation":{rotation},"timestamp":{timestamp}}}"#
        )),
        InputEvent::MouseMove { dx, dy, timestamp } => Some(format!(
            r#"{{"event_type":"mouse_moved","dx":{dx},"dy":{dy},"timestamp":{timestamp}}}"#
        )),
        InputEvent::AnalogDepth {
            rawcode,
            depth,
            timestamp,
        } => Some(format!(
            r#"{{"event_type":"analog_depth","rawcode":{rawcode},"depth":{depth:.4},"timestamp":{timestamp}}}"#
        )),
    }
}

async fn distributor(
    mut input_rx: mpsc::UnboundedReceiver<InputEvent>,
    bcast_tx: Arc<broadcast::Sender<Arc<str>>>,
    config: Arc<RwLock<Config>>,
) {
    let flush_interval = {
        let hz = config.read().await.flush_hz.max(1);
        Duration::from_micros(1_000_000 / hz as u64)
    };
    let mut last_flush = Instant::now();
    let mut pending_dx = 0i32;
    let mut pending_dy = 0i32;
    let mut pending_move_ts: u64 = 0;
    let mut pending: Vec<String> = Vec::new();

    loop {
        loop {
            match input_rx.try_recv() {
                Ok(event) => {
                    let cfg = config.read().await;
                    if !is_allowed(&event, &cfg) {
                        continue;
                    }
                    match event {
                        InputEvent::MouseMove { dx, dy, timestamp } => {
                            if pending_dx == 0 && pending_dy == 0 {
                                pending_move_ts = timestamp;
                            }
                            pending_dx += dx;
                            pending_dy += dy;
                        }
                        other => {
                            if let Some(json) = event_to_json(&other) {
                                pending.push(json);
                            }
                        }
                    }
                }
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => return,
            }
        }

        if last_flush.elapsed() >= flush_interval {
            for json in pending.drain(..) {
                let _ = bcast_tx.send(Arc::from(json.as_str()));
            }
            if pending_dx != 0 || pending_dy != 0 {
                let json = format!(
                    r#"{{"event_type":"mouse_moved","dx":{pending_dx},"dy":{pending_dy},"timestamp":{pending_move_ts}}}"#
                );
                let _ = bcast_tx.send(Arc::from(json.as_str()));
                pending_dx = 0;
                pending_dy = 0;
                pending_move_ts = 0;
            }
            last_flush = Instant::now();
        }

        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    addr: SocketAddr,
    config: Arc<RwLock<Config>>,
    bcast_tx: Arc<broadcast::Sender<Arc<str>>>,
    app_handle: tauri::AppHandle,
    status: Arc<Mutex<ServerStatus>>,
) {
    let ws = match accept_async(stream).await {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!("ws handshake failed from {addr}: {e}");
            return;
        }
    };
    let (mut write, mut read) = ws.split();

    //handshake first message needs to be {"type":"auth","token":"..."}
    let auth_token = config.read().await.auth_token.clone();

    let authed = loop {
        match read.next().await {
            Some(Ok(Message::Text(msg))) => {
                #[derive(Deserialize)]
                struct AuthMsg {
                    #[serde(rename = "type")]
                    msg_type: String,
                    token: Option<String>,
                }
                match serde_json::from_str::<AuthMsg>(&msg) {
                    Ok(m) if m.msg_type == "auth" => {
                        let token = m.token.unwrap_or_default();
                        if token.is_empty() {
                            let _ = write
                                .send(Message::Text(
                                    r#"{"type":"auth_response","status":"failed"}"#.into(),
                                ))
                                .await;
                            tracing::warn!("auth rejected from {addr}: no token");
                            break false;
                        } else if auth_token.is_empty() || token == auth_token {
                            let _ = write
                                .send(Message::Text(
                                    r#"{"type":"auth_response","status":"success"}"#.into(),
                                ))
                                .await;
                            tracing::info!("client authenticated from {addr}");
                            break true;
                        } else {
                            let _ = write
                                .send(Message::Text(
                                    r#"{"type":"auth_response","status":"failed"}"#.into(),
                                ))
                                .await;
                            tracing::warn!("auth rejected from {addr}: bad token");
                            break false;
                        }
                    }
                    _ => {
                        tracing::debug!("unexpected message from {addr}, ignoring");
                    }
                }
            }
            Some(Ok(Message::Close(_))) | None => break false,
            Some(Err(e)) => {
                tracing::warn!("ws error from {addr}: {e}");
                break false;
            }
            _ => {}
        }
    };

    if !authed {
        return;
    }

    {
        let mut s = status.lock().unwrap();
        s.clients.push(addr.to_string());
        s.client_count = s.clients.len();
        app_handle.emit("status-update", s.clone()).ok();
    }

    let mut rx = bcast_tx.subscribe();

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(json) => {
                        if write.send(Message::Text(json.to_string())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("client {addr} lagged by {n} messages");
                    }
                }
            }
            msg = read.next() => {
                match msg {
                    None | Some(Ok(Message::Close(_))) => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }

    {
        let mut s = status.lock().unwrap();
        s.clients.retain(|a| a != &addr.to_string());
        s.client_count = s.clients.len();
        app_handle.emit("status-update", s.clone()).ok();
    }
    tracing::info!("client disconnected: {addr}");
}

pub async fn run(
    config: Arc<RwLock<Config>>,
    input_rx: mpsc::UnboundedReceiver<InputEvent>,
    status: Arc<Mutex<ServerStatus>>,
    mut rebind_rx: watch::Receiver<()>,
    app_handle: tauri::AppHandle,
) {
    let (bcast_tx, _) = broadcast::channel::<Arc<str>>(1024);
    let bcast_tx = Arc::new(bcast_tx);

    {
        let cfg = Arc::clone(&config);
        let btx = Arc::clone(&bcast_tx);
        tokio::spawn(async move { distributor(input_rx, btx, cfg).await });
    }

    loop {
        let (host, port) = {
            let cfg = config.read().await;
            (cfg.host.clone(), cfg.port)
        };
        let bind_addr = format!("{host}:{port}");

        match TcpListener::bind(&bind_addr).await {
            Ok(listener) => {
                {
                    let mut s = status.lock().unwrap();
                    s.running = true;
                    s.bind_error = None;
                    s.host = host.clone();
                    s.port = port;
                }
                app_handle
                    .emit("status-update", status.lock().unwrap().clone())
                    .ok();
                tracing::info!("ws server listening on ws://{bind_addr}");

                loop {
                    tokio::select! {
                        conn = listener.accept() => {
                            match conn {
                                Ok((stream, addr)) => {
                                    let cfg = Arc::clone(&config);
                                    let btx = Arc::clone(&bcast_tx);
                                    let ah  = app_handle.clone();
                                    let st  = Arc::clone(&status);
                                    tokio::spawn(async move {
                                        handle_connection(stream, addr, cfg, btx, ah, st).await;
                                    });
                                }
                                Err(e) => tracing::warn!("accept error: {e}"),
                            }
                        }
                        _ = rebind_rx.changed() => break,
                    }
                }
            }
            Err(e) => {
                let kind = bind_error_kind(&e).to_string();
                tracing::error!("failed to bind {bind_addr}: {e} ({kind})");
                {
                    let mut s = status.lock().unwrap();
                    s.running = false;
                    s.bind_error = Some(kind);
                    s.host = host;
                    s.port = port;
                }
                app_handle
                    .emit("status-update", status.lock().unwrap().clone())
                    .ok();
                //wait for rebind
                let _ = rebind_rx.changed().await;
            }
        }
    }
}

fn bind_error_kind(e: &std::io::Error) -> &'static str {
    match e.kind() {
        std::io::ErrorKind::AddrInUse => "inuse",
        std::io::ErrorKind::PermissionDenied => "denied",
        _ => "oserror",
    }
}
