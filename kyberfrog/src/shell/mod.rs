// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native desktop shell (#21).
//!
//! On Windows this is a Tauri/WebView2 window pointing at the embedded web UI
//! (`http://localhost:<web_port>/`) — a native chrome around the *same* axum
//! server the browser uses, never an asset pipeline. Elsewhere it is the old
//! headless command loop, unchanged.
//!
//! Lifecycle (arbitrated in `docs/dev/plan-tauri-shell.md`):
//! * the window shows at startup;
//! * closing it only hides it — the tray keeps running;
//! * only the tray's "Quitter" (or Ctrl-C) stops the app, through the same
//!   shutdown sequence as before.

use std::sync::Arc;

use log::info;

use crate::app::{self, AppState};
use crate::tray::{TrayCommand, TrayHandle};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
use stub as imp;

pub use imp::run;

/// Everything `main` boots before handing the process over to the shell.
pub struct Boot {
    pub state: Arc<AppState>,
    pub web_task: tokio::task::JoinHandle<()>,
    pub tray_handle: Option<TrayHandle>,
    pub command_rx: tokio::sync::mpsc::Receiver<TrayCommand>,
    pub web_port: u16,
}

/// What a tray command means for the shell's own loop. Everything the app can
/// do by itself is applied inside [`dispatch`]; only the shell-owned outcomes
/// (window, process exit) bubble up.
pub enum Flow {
    Continue,
    OpenDashboard,
    Quit,
}

/// Apply one tray command to the app state.
pub async fn dispatch(command: TrayCommand, state: &Arc<AppState>) -> Flow {
    match command {
        TrayCommand::AddSpout { sender } => app::op_add_spout(state, sender, None).await,
        TrayCommand::AddScreen => app::op_add_screen(state, None).await,
        TrayCommand::RestartTx { name } => app::op_restart_transmitter(state, &name).await,
        TrayCommand::RemoveTx { name } => app::op_remove_transmitter(state, &name).await,
        TrayCommand::StartViewer { id } => app::op_start_viewer(state, &id).await,
        TrayCommand::StopViewer { id } => app::op_stop_viewer(state, &id).await,
        TrayCommand::RestartViewer { id } => app::op_restart_viewer(state, &id).await,
        TrayCommand::RemoveViewer { id } => app::op_remove_viewer(state, &id).await,
        TrayCommand::OpenDashboard => return Flow::OpenDashboard,
        TrayCommand::Quit => {
            info!("Quit requested from tray");
            return Flow::Quit;
        }
    }
    Flow::Continue
}

/// The common shutdown sequence, shared by both shells: goodbye packets, kill
/// the supervised children, stop the web server, join the tray thread.
pub async fn shutdown(
    state: &Arc<AppState>,
    web_task: &tokio::task::JoinHandle<()>,
    tray_handle: &mut Option<TrayHandle>,
) {
    info!("Shutting down");
    if let Some(discovery) = &state.discovery {
        discovery.shutdown(); // goodbye packets before the children die
    }
    state.manager.lock().await.shutdown_all().await;
    web_task.abort();
    if let Some(mut handle) = tray_handle.take() {
        handle.shutdown().await;
    }
}
