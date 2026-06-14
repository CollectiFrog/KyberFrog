// SPDX-License-Identifier: AGPL-3.0-or-later

//! The client's embedded web UI and control endpoint.
//!
//! Serves a dashboard (machine identity, the kyclient instances, and logs) plus
//! a small JSON/REST API to create, edit, start, stop, restart and remove
//! viewers at runtime. Every change is persisted to `client-agent.toml`, so the
//! agent relaunches `enabled` instances on the next boot. Bound on all
//! interfaces — trusted LAN, no auth on the UI itself (see IMPROVEMENTS.md).

use std::net::SocketAddr;
use std::path::Path as FsPath;
use std::sync::Arc;

use axum::extract::{Path, Query, State as AxState};
use axum::response::Html;
use axum::routing::{get, post};
use axum::{Json, Router};
use log::{error, info};
use serde::{Deserialize, Serialize};
use shared::paths;
use tokio::sync::Mutex;

use crate::config::{self, ClientConfig, Instance};
use crate::supervisor::{Manager, State, StatusMap};

const DASHBOARD_HTML: &str = include_str!("web/index.html");

/// Shared application state handed to every handler.
pub struct AppState {
    pub config: Mutex<ClientConfig>,
    pub manager: Mutex<Manager>,
    pub status: StatusMap,
}

/// Spawn the web server task. It runs until the process exits.
pub fn spawn(state: Arc<AppState>, port: u16) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let app = Router::new()
            .route("/", get(dashboard))
            .route("/status", get(status_handler))
            .route("/logs/:source", get(logs_handler))
            .route("/instances", post(create_instance))
            .route("/instances/:id", post(update_instance).delete(remove_instance))
            .route("/instances/:id/start", post(start_instance))
            .route("/instances/:id/stop", post(stop_instance))
            .route("/instances/:id/restart", post(restart_instance))
            .with_state(state);

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(err) => {
                error!("Web UI disabled: cannot bind {addr}: {err}");
                return;
            }
        };

        info!("Web UI on http://localhost:{port}/ (and the machine's LAN IP)");
        if let Err(err) = axum::serve(listener, app).await {
            error!("Web server stopped: {err}");
        }
    })
}

// ---------------------------------------------------------------------------
// Payloads
// ---------------------------------------------------------------------------

/// Body of create / update requests.
#[derive(Deserialize)]
struct InstanceForm {
    server: String,
    port: u16,
    #[serde(default = "default_true")]
    fullscreen: bool,
}

#[derive(Serialize)]
struct InstanceView {
    id: String,
    server: String,
    port: u16,
    fullscreen: bool,
    enabled: bool,
    status: &'static str,
}

#[derive(Serialize)]
struct StatusResponse {
    hostname: String,
    ips: Vec<String>,
    instances: Vec<InstanceView>,
}

#[derive(Deserialize)]
struct LogQuery {
    lines: Option<usize>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

async fn status_handler(AxState(state): AxState<Arc<AppState>>) -> Json<StatusResponse> {
    Json(build_status(&state).await)
}

async fn logs_handler(
    Path(source): Path<String>,
    Query(query): Query<LogQuery>,
) -> String {
    // Only "client" or a safe instance id (alphanumeric / dash); never a path.
    let path = if source == "client" {
        paths::client_log_file()
    } else if source.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        paths::kyclient_log_file(&source)
    } else {
        return String::new();
    };
    let lines = query.lines.unwrap_or(200).min(2000);
    tail(&path, lines)
}

async fn create_instance(
    AxState(state): AxState<Arc<AppState>>,
    Json(form): Json<InstanceForm>,
) -> Json<StatusResponse> {
    let instance = {
        let mut config = state.config.lock().await;
        let instance = Instance {
            id: config.unique_id(),
            server: form.server,
            port: form.port,
            fullscreen: form.fullscreen,
            enabled: true,
        };
        config.instances.push(instance.clone());
        persist(&config);
        instance
    };
    state.manager.lock().await.start(&instance);
    Json(build_status(&state).await)
}

async fn update_instance(
    AxState(state): AxState<Arc<AppState>>,
    Path(id): Path<String>,
    Json(form): Json<InstanceForm>,
) -> Json<StatusResponse> {
    let instance = {
        let mut config = state.config.lock().await;
        if let Some(inst) = config.get_mut(&id) {
            inst.server = form.server;
            inst.port = form.port;
            inst.fullscreen = form.fullscreen;
        }
        let cloned = config.get(&id).cloned();
        persist(&config);
        cloned
    };
    if let Some(instance) = instance {
        if instance.enabled {
            state.manager.lock().await.restart(&instance).await;
        }
    }
    Json(build_status(&state).await)
}

async fn start_instance(
    AxState(state): AxState<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<StatusResponse> {
    let instance = {
        let mut config = state.config.lock().await;
        if let Some(inst) = config.get_mut(&id) {
            inst.enabled = true;
        }
        let cloned = config.get(&id).cloned();
        persist(&config);
        cloned
    };
    if let Some(instance) = instance {
        state.manager.lock().await.start(&instance);
    }
    Json(build_status(&state).await)
}

async fn stop_instance(
    AxState(state): AxState<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<StatusResponse> {
    {
        let mut config = state.config.lock().await;
        if let Some(inst) = config.get_mut(&id) {
            inst.enabled = false;
        }
        persist(&config);
    }
    state.manager.lock().await.stop(&id).await;
    Json(build_status(&state).await)
}

async fn restart_instance(
    AxState(state): AxState<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<StatusResponse> {
    let instance = {
        let config = state.config.lock().await;
        config.get(&id).cloned()
    };
    if let Some(instance) = instance {
        state.manager.lock().await.restart(&instance).await;
    }
    Json(build_status(&state).await)
}

async fn remove_instance(
    AxState(state): AxState<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<StatusResponse> {
    state.manager.lock().await.stop(&id).await;
    {
        let mut config = state.config.lock().await;
        config.instances.retain(|i| i.id != id);
        persist(&config);
    }
    Json(build_status(&state).await)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async fn build_status(state: &AppState) -> StatusResponse {
    let hostname = hostname();
    let ips = local_ips();
    let config = state.config.lock().await;
    let status = state.status.lock().ok();
    let instances = config
        .instances
        .iter()
        .map(|i| InstanceView {
            id: i.id.clone(),
            server: i.server.clone(),
            port: i.port,
            fullscreen: i.fullscreen,
            enabled: i.enabled,
            status: status
                .as_ref()
                .and_then(|m| m.get(&i.id).copied())
                .map(State::as_str)
                .unwrap_or("stopped"),
        })
        .collect();
    StatusResponse {
        hostname,
        ips,
        instances,
    }
}

fn persist(config: &ClientConfig) {
    if let Err(err) = config::save(config) {
        error!("Failed to persist client config: {err:#}");
    }
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

/// Last `n` lines of the file at `path` (empty if it doesn't exist yet).
fn tail(path: &FsPath, n: usize) -> String {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().collect();
            let start = lines.len().saturating_sub(n);
            lines[start..].join("\n")
        }
        Err(_) => String::new(),
    }
}

fn default_true() -> bool {
    true
}
