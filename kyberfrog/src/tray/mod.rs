// SPDX-License-Identifier: AGPL-3.0-or-later

//! Unified system-tray UI.
//!
//! One tray, one icon. Its context menu is rebuilt on every click from the
//! shared [`TrayModel`] and shows **both** roles:
//!
//! * Émission — one entry per transmitter (Restart / Remove) plus an "Add
//!   transmitter" picker fed by a *live* Spout-sender enumeration.
//! * Réception — one entry per viewer (Start / Stop / Restart / Remove).
//!
//! Interaction is one-way in each direction:
//! * tray thread → app: [`TrayCommand`]s over a tokio mpsc channel.
//! * app → tray thread: mutations of the shared [`TrayModel`] (read on the next
//!   menu open).

use std::sync::{Arc, Mutex};

use shared::{Transmitter, Viewer};

use crate::supervisor::StatusMap;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
use stub as imp;

pub use imp::{spawn, TrayHandle};

/// A command emitted by the tray, consumed by the app's main loop.
#[derive(Clone, Debug)]
pub enum TrayCommand {
    /// Create a transmitter pinned to a Spout sender.
    AddSpout { sender: String },
    /// Create a plain screen-capture transmitter.
    AddScreen,
    /// Restart the named transmitter.
    RestartTx { name: String },
    /// Remove (stop + forget) the named transmitter.
    RemoveTx { name: String },
    /// Start the named viewer.
    StartViewer { id: String },
    /// Stop the named viewer.
    StopViewer { id: String },
    /// Restart the named viewer.
    RestartViewer { id: String },
    /// Remove (stop + forget) the named viewer.
    RemoveViewer { id: String },
    /// Quit the app.
    Quit,
}

/// State the tray reads (on menu open) to render itself.
pub struct TrayModel {
    transmitters: Mutex<Vec<Transmitter>>,
    viewers: Mutex<Vec<Viewer>>,
    pub status: StatusMap,
    pub web_port: u16,
}

impl TrayModel {
    pub fn new(
        transmitters: Vec<Transmitter>,
        viewers: Vec<Viewer>,
        status: StatusMap,
        web_port: u16,
    ) -> Arc<Self> {
        Arc::new(Self {
            transmitters: Mutex::new(transmitters),
            viewers: Mutex::new(viewers),
            status,
            web_port,
        })
    }

    /// Replace the transmitter list shown by the tray.
    pub fn set_transmitters(&self, transmitters: Vec<Transmitter>) {
        if let Ok(mut guard) = self.transmitters.lock() {
            *guard = transmitters;
        }
    }

    /// Replace the viewer list shown by the tray.
    pub fn set_viewers(&self, viewers: Vec<Viewer>) {
        if let Ok(mut guard) = self.viewers.lock() {
            *guard = viewers;
        }
    }

    /// Snapshot of the transmitters for menu rendering.
    #[cfg_attr(not(windows), allow(dead_code))]
    fn transmitters_snapshot(&self) -> Vec<Transmitter> {
        self.transmitters.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// Snapshot of the viewers for menu rendering.
    #[cfg_attr(not(windows), allow(dead_code))]
    fn viewers_snapshot(&self) -> Vec<Viewer> {
        self.viewers.lock().map(|g| g.clone()).unwrap_or_default()
    }
}
