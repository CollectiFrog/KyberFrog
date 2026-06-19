// SPDX-License-Identifier: AGPL-3.0-or-later

//! The unified embedded web UI (single port, default 7700).
//!
//! Serves one dashboard with two panels — Émission (transmitters) and Réception
//! (viewers) — plus the live logs, and a small JSON API to drive both halves.
//! `GET /transmitters` stays as the stable discovery endpoint other instances
//! poll. Bound on all interfaces — trusted LAN, no auth on the UI itself (see
//! IMPROVEMENTS.md).

use std::net::SocketAddr;
use std::path::Path as FsPath;
use std::sync::Arc;

use axum::extract::{Path, Query, State as AxState};
use axum::response::Html;
use axum::routing::{get, post};
use axum::{Json, Router};
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use shared::paths;

use crate::app::{self, AppState, StatusPayload, TxView};
use crate::spout;

const DASHBOARD_HTML: &str = include_str!("web/index.html");

/// Spawn the web server task. It runs until the process exits.
pub fn spawn(state: Arc<AppState>, port: u16) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let app = Router::new()
            .route("/", get(dashboard))
            .route("/status", get(status_handler))
            .route("/transmitters", get(transmitters).post(create_transmitter))
            .route("/transmitters/:name/restart", post(restart_transmitter))
            .route("/transmitters/:name", axum::routing::delete(remove_transmitter))
            .route("/spout-senders", get(spout_senders))
            .route("/viewers", post(create_viewer))
            .route("/viewers/:id", post(update_viewer).delete(remove_viewer))
            .route("/viewers/:id/start", post(start_viewer))
            .route("/viewers/:id/stop", post(stop_viewer))
            .route("/viewers/:id/restart", post(restart_viewer))
            .route("/logs/app", get(logs_app))
            .route("/logs/transmitter/:name", get(logs_transmitter))
            .route("/logs/viewer/:id", get(logs_viewer))
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

/// Body of `POST /transmitters`.
#[derive(Deserialize)]
struct AddTransmitterForm {
    /// `"spout"` or `"screen"`.
    kind: String,
    /// Required for `"spout"`.
    #[serde(default)]
    sender: Option<String>,
    /// Optional explicit control-plane port; auto-allocated when omitted/0.
    #[serde(default)]
    port: Option<u16>,
}

/// Body of viewer create / update requests.
#[derive(Deserialize)]
struct ViewerForm {
    /// Optional viewer id (name): chosen at create, or the new id on rename.
    #[serde(default)]
    id: Option<String>,
    server: String,
    port: u16,
    #[serde(default = "default_true")]
    fullscreen: bool,
    /// Optional Spout sender name → windowless relay (empty/absent = off).
    #[serde(default)]
    spout_out: Option<String>,
    /// Remote-control viewer: windowed, forwards keyboard + mouse. Mutually
    /// exclusive with `spout_out`.
    #[serde(default)]
    remote_control: bool,
}

#[derive(Serialize)]
struct SendersView {
    names: Vec<String>,
    active: Option<String>,
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

async fn status_handler(AxState(state): AxState<Arc<AppState>>) -> Json<StatusPayload> {
    Json(state.status_payload().await)
}

/// `GET /transmitters` — discovery JSON (list + status), unchanged shape.
async fn transmitters(AxState(state): AxState<Arc<AppState>>) -> Json<Vec<TxView>> {
    Json(state.transmitter_views().await)
}

async fn create_transmitter(
    AxState(state): AxState<Arc<AppState>>,
    Json(form): Json<AddTransmitterForm>,
) -> Json<StatusPayload> {
    match form.kind.as_str() {
        "spout" => match form.sender {
            Some(sender) if !sender.trim().is_empty() => {
                app::op_add_spout(&state, sender, form.port).await
            }
            _ => warn!("create_transmitter: spout kind without a sender name"),
        },
        "screen" => app::op_add_screen(&state, form.port).await,
        other => warn!("create_transmitter: unknown kind {other:?}"),
    }
    Json(state.status_payload().await)
}

async fn restart_transmitter(
    AxState(state): AxState<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<StatusPayload> {
    app::op_restart_transmitter(&state, &name).await;
    Json(state.status_payload().await)
}

async fn remove_transmitter(
    AxState(state): AxState<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<StatusPayload> {
    app::op_remove_transmitter(&state, &name).await;
    Json(state.status_payload().await)
}

async fn spout_senders() -> Json<SendersView> {
    let senders = spout::list_senders();
    Json(SendersView {
        names: senders.names,
        active: senders.active,
    })
}

async fn create_viewer(
    AxState(state): AxState<Arc<AppState>>,
    Json(form): Json<ViewerForm>,
) -> Json<StatusPayload> {
    app::op_add_viewer(
        &state,
        form.id,
        form.server,
        form.port,
        form.fullscreen,
        form.spout_out,
        form.remote_control,
    )
    .await;
    Json(state.status_payload().await)
}

async fn update_viewer(
    AxState(state): AxState<Arc<AppState>>,
    Path(id): Path<String>,
    Json(form): Json<ViewerForm>,
) -> Json<StatusPayload> {
    app::op_update_viewer(
        &state,
        &id,
        form.id,
        form.server,
        form.port,
        form.fullscreen,
        form.spout_out,
        form.remote_control,
    )
    .await;
    Json(state.status_payload().await)
}

async fn start_viewer(
    AxState(state): AxState<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<StatusPayload> {
    app::op_start_viewer(&state, &id).await;
    Json(state.status_payload().await)
}

async fn stop_viewer(
    AxState(state): AxState<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<StatusPayload> {
    app::op_stop_viewer(&state, &id).await;
    Json(state.status_payload().await)
}

async fn restart_viewer(
    AxState(state): AxState<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<StatusPayload> {
    app::op_restart_viewer(&state, &id).await;
    Json(state.status_payload().await)
}

async fn remove_viewer(
    AxState(state): AxState<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<StatusPayload> {
    app::op_remove_viewer(&state, &id).await;
    Json(state.status_payload().await)
}

async fn logs_app(Query(query): Query<LogQuery>) -> String {
    tail(&paths::app_log_file(), lines(&query))
}

async fn logs_transmitter(Path(name): Path<String>, Query(query): Query<LogQuery>) -> String {
    if !is_safe(&name) {
        return String::new();
    }
    tail(&paths::kycontroller_log_file(&name), lines(&query))
}

async fn logs_viewer(Path(id): Path<String>, Query(query): Query<LogQuery>) -> String {
    if !is_safe(&id) {
        return String::new();
    }
    tail(&paths::kyclient_log_file(&id), lines(&query))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A name/id is safe to turn into a log path when it is purely alphanumeric or
/// dashes (never a path component).
fn is_safe(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn lines(query: &LogQuery) -> usize {
    query.lines.unwrap_or(200).min(2000)
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
