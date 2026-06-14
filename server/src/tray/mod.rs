// SPDX-License-Identifier: AGPL-3.0-or-later

//! System-tray UI for the Server.
//!
//! The tray runs on a dedicated OS thread with a Windows message pump (muda +
//! `Shell_NotifyIconW`). Its context menu is rebuilt on every click from the
//! shared [`TrayModel`] plus a *live* enumeration of Spout senders, so the
//! "Add transmitter" picker always reflects what is currently being published.
//!
//! Interaction is one-way in each direction:
//! * tray thread → Server: [`TrayCommand`]s over a tokio mpsc channel.
//! * Server → tray thread: mutations of the shared [`TrayModel`] (read on the
//!   next menu open). No status push is needed.

use std::sync::{Arc, Mutex};

use shared::Transmitter;

use crate::supervisor::{State, StatusMap};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
use stub as imp;

pub use imp::{spawn, TrayHandle};

/// A command emitted by the tray, consumed by the Server's main loop.
#[derive(Clone, Debug)]
pub enum TrayCommand {
    /// Create a transmitter pinned to a Spout sender.
    AddSpout { sender: String },
    /// Create a plain screen-capture transmitter.
    AddScreen,
    /// Remove (stop + forget) the named transmitter.
    Remove { name: String },
    /// Restart the named transmitter.
    Restart { name: String },
    /// Quit the Server.
    Quit,
}

/// State the tray reads (on menu open) to render itself.
pub struct TrayModel {
    transmitters: Mutex<Vec<Transmitter>>,
    status: StatusMap,
}

impl TrayModel {
    pub fn new(transmitters: Vec<Transmitter>, status: StatusMap) -> Arc<Self> {
        Arc::new(Self {
            transmitters: Mutex::new(transmitters),
            status,
        })
    }

    /// Replace the transmitter list shown by the tray.
    pub fn set_transmitters(&self, transmitters: Vec<Transmitter>) {
        if let Ok(mut guard) = self.transmitters.lock() {
            *guard = transmitters;
        }
    }

    /// Snapshot of the transmitters for menu rendering.
    #[cfg_attr(not(windows), allow(dead_code))]
    fn transmitters_snapshot(&self) -> Vec<Transmitter> {
        self.transmitters
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    /// Snapshot of each transmitter joined with its current supervision state,
    /// for the web UI / discovery endpoint.
    pub(crate) fn snapshot(&self) -> Vec<(Transmitter, Option<State>)> {
        let transmitters = self.transmitters_snapshot();
        let status = self.status.lock().ok();
        transmitters
            .into_iter()
            .map(|t| {
                let state = status.as_ref().and_then(|m| m.get(&t.name).copied());
                (t, state)
            })
            .collect()
    }
}
