// SPDX-License-Identifier: AGPL-3.0-or-later

//! Shared application state and the operations the web UI and the tray both
//! drive.
//!
//! Every mutation goes through one of the `op_*` functions here so the two
//! front-ends stay in lockstep: each locks the config, applies the change to
//! the running [`Manager`], persists `kyberfrog.toml`, and refreshes the tray's
//! render snapshot. Locks are always taken **config before manager** to avoid
//! deadlock.

use std::sync::Arc;

use log::{error, warn};
use serde::Serialize;
use shared::config::{self, Config};
use shared::{Source, Transmitter, Viewer};
use tokio::sync::Mutex;

use crate::supervisor::{state_of, Key, Manager, StatusMap};
use crate::tray::TrayModel;

/// State shared by every web handler and the tray-command loop.
pub struct AppState {
    pub config: Mutex<Config>,
    pub manager: Mutex<Manager>,
    pub status: StatusMap,
    pub tray_model: Arc<TrayModel>,
}

// ---------------------------------------------------------------------------
// Status payload (web GET /status) + discovery view (GET /transmitters)
// ---------------------------------------------------------------------------

/// One transmitter over HTTP: its config fields flattened plus current status.
#[derive(Serialize)]
pub struct TxView {
    #[serde(flatten)]
    transmitter: Transmitter,
    status: &'static str,
}

/// One viewer over HTTP.
#[derive(Serialize)]
pub struct ViewerView {
    id: String,
    server: String,
    port: u16,
    fullscreen: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    spout_out: Option<String>,
    remote_control: bool,
    enabled: bool,
    status: &'static str,
}

/// The dashboard payload: machine identity plus both halves with live status.
#[derive(Serialize)]
pub struct StatusPayload {
    hostname: String,
    ips: Vec<String>,
    transmitters: Vec<TxView>,
    viewers: Vec<ViewerView>,
}

impl AppState {
    /// Snapshot of every transmitter joined with its supervision status.
    pub async fn transmitter_views(&self) -> Vec<TxView> {
        let config = self.config.lock().await;
        let status = self.status.lock().ok();
        config
            .emission
            .transmitters
            .iter()
            .map(|t| TxView {
                transmitter: t.clone(),
                status: status
                    .as_ref()
                    .map(|m| state_of(m, &Key::Tx(t.name.clone())).as_str())
                    .unwrap_or("unknown"),
            })
            .collect()
    }

    /// The full dashboard payload.
    pub async fn status_payload(&self) -> StatusPayload {
        let hostname = hostname();
        let ips = local_ips();
        let config = self.config.lock().await;
        let status = self.status.lock().ok();

        let transmitters = config
            .emission
            .transmitters
            .iter()
            .map(|t| TxView {
                transmitter: t.clone(),
                status: status
                    .as_ref()
                    .map(|m| state_of(m, &Key::Tx(t.name.clone())).as_str())
                    .unwrap_or("unknown"),
            })
            .collect();

        let viewers = config
            .reception
            .viewers
            .iter()
            .map(|v| ViewerView {
                id: v.id.clone(),
                server: v.server.clone(),
                port: v.port,
                fullscreen: v.fullscreen,
                spout_out: v.spout_out.clone(),
                remote_control: v.remote_control,
                enabled: v.enabled,
                status: status
                    .as_ref()
                    .map(|m| state_of(m, &Key::Vw(v.id.clone())).as_str())
                    .unwrap_or("stopped"),
            })
            .collect();

        StatusPayload {
            hostname,
            ips,
            transmitters,
            viewers,
        }
    }
}

// ---------------------------------------------------------------------------
// Operations — transmitters (emission)
// ---------------------------------------------------------------------------

/// Create a transmitter pinned to a Spout sender, start it, persist it.
/// `port` is honored when given (and free), otherwise auto-allocated.
pub async fn op_add_spout(state: &AppState, sender: String, port: Option<u16>) {
    let mut config = state.config.lock().await;
    let Some(port) = resolve_port(&config, port) else {
        return;
    };
    let name = unique_name(&sender, &config);
    let tx = Transmitter {
        name,
        port,
        source: Source::Spout { sender },
    };
    add_transmitter(state, &mut config, tx).await;
}

/// Create a plain screen-capture transmitter, start it, persist it.
/// `port` is honored when given (and free), otherwise auto-allocated.
pub async fn op_add_screen(state: &AppState, port: Option<u16>) {
    let mut config = state.config.lock().await;
    let Some(port) = resolve_port(&config, port) else {
        return;
    };
    let name = unique_name("screen", &config);
    let tx = Transmitter {
        name,
        port,
        source: Source::Screen { display: None },
    };
    add_transmitter(state, &mut config, tx).await;
}

/// Start `tx`; only persist it into the config if it actually started.
async fn add_transmitter(state: &AppState, config: &mut Config, tx: Transmitter) {
    {
        let mut manager = state.manager.lock().await;
        if let Err(err) = manager.start_transmitter(&tx) {
            error!("Failed to start new transmitter {:?}: {err:#}", tx.name);
            return;
        }
    }
    config.emission.transmitters.push(tx);
    persist_and_refresh(config, &state.tray_model);
}

/// Restart the named transmitter (regenerates its config). No config change.
pub async fn op_restart_transmitter(state: &AppState, name: &str) {
    let tx = {
        let config = state.config.lock().await;
        config.emission.get(name).cloned()
    };
    let Some(tx) = tx else {
        warn!("Restart requested for unknown transmitter {name:?}");
        return;
    };
    let mut manager = state.manager.lock().await;
    if let Err(err) = manager.restart_transmitter(&tx).await {
        error!("Failed to restart transmitter {name:?}: {err:#}");
    }
}

/// Stop and forget the named transmitter.
pub async fn op_remove_transmitter(state: &AppState, name: &str) {
    let mut config = state.config.lock().await;
    state.manager.lock().await.stop_transmitter(name).await;
    config.emission.transmitters.retain(|t| t.name != name);
    persist_and_refresh(&config, &state.tray_model);
}

// ---------------------------------------------------------------------------
// Operations — viewers (reception)
// ---------------------------------------------------------------------------

/// Create and start a viewer. `requested_id` (the viewer name) is honored when
/// valid and free, otherwise an auto id (`viewer-N`) is used.
pub async fn op_add_viewer(
    state: &AppState,
    requested_id: Option<String>,
    server: String,
    port: u16,
    fullscreen: bool,
    spout_out: Option<String>,
    remote_control: bool,
) {
    let viewer = {
        let mut config = state.config.lock().await;
        let viewer = Viewer {
            id: resolve_viewer_id(&config, requested_id, None),
            server,
            port,
            fullscreen,
            // Remote control (windowed + inputs) and Spout relay (windowless)
            // are mutually exclusive; remote control wins and drops any Spout.
            spout_out: if remote_control { None } else { normalize_spout(spout_out) },
            remote_control,
            enabled: true,
        };
        config.reception.viewers.push(viewer.clone());
        persist_and_refresh(&config, &state.tray_model);
        viewer
    };
    state.manager.lock().await.start_viewer(&viewer);
}

/// Apply edited fields to a viewer, optionally **renaming** it (`new_id`), and
/// hot-relaunch it if enabled. A rename stops the old child and starts the new
/// one (new id → new log file). An invalid/taken `new_id` keeps the old id.
pub async fn op_update_viewer(
    state: &AppState,
    id: &str,
    new_id: Option<String>,
    server: String,
    port: u16,
    fullscreen: bool,
    spout_out: Option<String>,
    remote_control: bool,
) {
    let (renamed, updated) = {
        let mut config = state.config.lock().await;
        if config.reception.get(id).is_none() {
            warn!("Update requested for unknown viewer {id:?}");
            return;
        }
        let target_id = resolve_viewer_id(&config, new_id, Some(id));
        let renamed = target_id != id;
        if let Some(v) = config.reception.get_mut(id) {
            v.server = server;
            v.port = port;
            v.fullscreen = fullscreen;
            // Remote control and Spout relay are mutually exclusive.
            v.spout_out = if remote_control { None } else { normalize_spout(spout_out) };
            v.remote_control = remote_control;
            v.id = target_id.clone();
        }
        let updated = config.reception.get(&target_id).cloned();
        persist_and_refresh(&config, &state.tray_model);
        (renamed, updated)
    };

    let Some(updated) = updated else { return };
    let mut manager = state.manager.lock().await;
    if renamed {
        // Tear down the old-named child; start the new one if it should run.
        manager.stop_viewer(id).await;
        if updated.enabled {
            manager.start_viewer(&updated);
        }
    } else if updated.enabled {
        manager.restart_viewer(&updated).await;
    }
}

/// Mark a viewer enabled and start it.
pub async fn op_start_viewer(state: &AppState, id: &str) {
    let viewer = {
        let mut config = state.config.lock().await;
        if let Some(v) = config.reception.get_mut(id) {
            v.enabled = true;
        }
        let cloned = config.reception.get(id).cloned();
        persist_and_refresh(&config, &state.tray_model);
        cloned
    };
    if let Some(viewer) = viewer {
        state.manager.lock().await.start_viewer(&viewer);
    } else {
        warn!("Start requested for unknown viewer {id:?}");
    }
}

/// Mark a viewer disabled and stop it.
pub async fn op_stop_viewer(state: &AppState, id: &str) {
    {
        let mut config = state.config.lock().await;
        if let Some(v) = config.reception.get_mut(id) {
            v.enabled = false;
        }
        persist_and_refresh(&config, &state.tray_model);
    }
    state.manager.lock().await.stop_viewer(id).await;
}

/// Restart a viewer in place (no config change).
pub async fn op_restart_viewer(state: &AppState, id: &str) {
    let viewer = {
        let config = state.config.lock().await;
        config.reception.get(id).cloned()
    };
    let Some(viewer) = viewer else {
        warn!("Restart requested for unknown viewer {id:?}");
        return;
    };
    state.manager.lock().await.restart_viewer(&viewer).await;
}

/// Stop and forget a viewer.
pub async fn op_remove_viewer(state: &AppState, id: &str) {
    state.manager.lock().await.stop_viewer(id).await;
    let mut config = state.config.lock().await;
    config.reception.viewers.retain(|v| v.id != id);
    persist_and_refresh(&config, &state.tray_model);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Save the config and refresh the tray's render snapshots.
fn persist_and_refresh(config: &Config, tray: &TrayModel) {
    if let Err(err) = config::save(config) {
        error!("Failed to persist config: {err:#}");
    }
    tray.set_transmitters(config.emission.transmitters.clone());
    tray.set_viewers(config.reception.viewers.clone());
}

/// A filesystem-safe, unique transmitter name derived from `base`.
fn unique_name(base: &str, config: &Config) -> String {
    let mut candidate = sanitize(base);
    if candidate.is_empty() {
        candidate = "tx".to_string();
    }
    if config.emission.get(&candidate).is_none() {
        return candidate;
    }
    let mut i = 2;
    loop {
        let next = format!("{candidate}-{i}");
        if config.emission.get(&next).is_none() {
            return next;
        }
        i += 1;
    }
}

/// Lowercase, collapse runs of non-alphanumerics to single `-`, trim edges.
fn sanitize(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Resolve the port for a new transmitter: honor an explicit, non-zero request
/// (rejecting a clash with an existing transmitter → `None`), otherwise
/// auto-allocate.
fn resolve_port(config: &Config, requested: Option<u16>) -> Option<u16> {
    match requested {
        Some(p) if p != 0 => {
            if config.emission.port_in_use(p, None) {
                warn!("Requested transmitter port {p} is already used by another transmitter");
                None
            } else {
                Some(p)
            }
        }
        _ => Some(allocate_port(config)),
    }
}

/// Resolve the id (name) for a viewer. An explicit, valid, unique `requested`
/// id wins; otherwise keep `current` (on update) or auto-allocate `viewer-N`
/// (on create). A valid id is non-empty and only `[A-Za-z0-9-]` (it is a URL
/// segment and a log file name).
fn resolve_viewer_id(config: &Config, requested: Option<String>, current: Option<&str>) -> String {
    if let Some(req) = requested {
        let req = req.trim();
        if !req.is_empty() {
            let taken = config
                .reception
                .viewers
                .iter()
                .any(|v| v.id == req && Some(v.id.as_str()) != current);
            if is_valid_viewer_id(req) && !taken {
                return req.to_string();
            }
            warn!("Viewer id {req:?} is invalid or already taken; keeping the previous id");
        }
    }
    match current {
        Some(c) => c.to_string(),
        None => config.reception.unique_id(),
    }
}

/// `true` if `s` is safe as a viewer id (URL segment + log file name).
fn is_valid_viewer_id(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Normalize a requested Spout sender name: trim, treat empty as "no Spout".
/// A non-empty name turns the viewer into a windowless Spout relay.
fn normalize_spout(requested: Option<String>) -> Option<String> {
    requested
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Pick a control-plane port for a new transmitter: the lowest free port at or
/// above `base_port` that no other transmitter uses and that can be bound now.
fn allocate_port(config: &Config) -> u16 {
    let mut port = config.emission.base_port;
    loop {
        port = config.emission.next_free_port(port);
        if port_is_available(port) {
            return port;
        }
        warn!("Port {port} is already in use, trying the next one");
        match port.checked_add(1) {
            Some(next) => port = next,
            None => return config.emission.base_port,
        }
    }
}

/// Best-effort check that `port` can currently be bound for TCP.
fn port_is_available(port: u16) -> bool {
    std::net::TcpListener::bind(("0.0.0.0", port)).is_ok()
}

/// This machine's name (Windows `COMPUTERNAME`, else `HOSTNAME`).
fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

/// The primary outbound IPv4 (via a connected-but-silent UDP socket).
fn local_ips() -> Vec<String> {
    let mut ips = Vec::new();
    if let Ok(sock) = std::net::UdpSocket::bind("0.0.0.0:0") {
        if sock.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = sock.local_addr() {
                ips.push(addr.ip().to_string());
            }
        }
    }
    ips
}
