// SPDX-License-Identifier: AGPL-3.0-or-later

//! The Director's embedded web UI and `/transmitters` discovery endpoint.
//!
//! Read-only for now: it serves one dashboard page that polls
//! `GET /transmitters`, which returns the live transmitter list joined with
//! each one's supervision state. Bound on all interfaces so other machines on
//! the LAN (operators, scene agents) can reach it. Runtime control (add /
//! remove / restart over HTTP) is a later increment.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State as AxumState;
use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use log::{error, info};
use serde::Serialize;
use shared::Transmitter;

use crate::supervisor::State;
use crate::tray::TrayModel;

/// The dashboard page, embedded so the binary stays self-contained.
const DASHBOARD_HTML: &str = include_str!("web/index.html");

/// Spawn the web server task. It runs until the process exits.
pub fn spawn(model: Arc<TrayModel>, port: u16) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let app = Router::new()
            .route("/", get(dashboard))
            .route("/transmitters", get(transmitters))
            .with_state(model);

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

/// `GET /` — the dashboard page.
async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

/// `GET /transmitters` — the live list, for the dashboard and for discovery.
async fn transmitters(
    AxumState(model): AxumState<Arc<TrayModel>>,
) -> Json<Vec<TransmitterView>> {
    let views = model
        .snapshot()
        .into_iter()
        .map(|(transmitter, state)| TransmitterView {
            transmitter,
            status: state.map(State::as_str).unwrap_or("unknown"),
        })
        .collect();
    Json(views)
}

/// One transmitter over HTTP: its config fields (name, port, structured source)
/// flattened, plus its current supervision status.
#[derive(Serialize)]
struct TransmitterView {
    #[serde(flatten)]
    transmitter: Transmitter,
    status: &'static str,
}
