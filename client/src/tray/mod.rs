// SPDX-License-Identifier: AGPL-3.0-or-later

//! System-tray UI for the KyberFrog Client.
//!
//! The tray runs on a dedicated OS thread with a Windows message pump. Its
//! context menu is rebuilt on every click from the shared [`TrayModel`] —
//! one submenu per kyclient instance showing its live state plus Start / Stop /
//! Restart / Remove actions. "Ouvrir dashboard" opens the web UI in the
//! default browser; "Quitter" shuts everything down.
//!
//! Interaction is one-way in each direction:
//! * tray thread → Client: [`TrayCommand`]s over a tokio mpsc channel.
//! * Client → tray thread: mutations of the shared [`TrayModel`].

use std::sync::{Arc, Mutex};

use crate::config::Instance;
use crate::supervisor::StatusMap;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as imp;

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
use stub as imp;

pub use imp::spawn;

/// A command emitted by the tray, consumed by the Client's main loop.
#[derive(Clone, Debug)]
pub enum TrayCommand {
    Start { id: String },
    Stop { id: String },
    Restart { id: String },
    Remove { id: String },
    Quit,
}

/// State the tray reads (on menu open) to render itself.
pub struct TrayModel {
    instances: Mutex<Vec<Instance>>,
    pub status: StatusMap,
    pub web_port: u16,
}

impl TrayModel {
    pub fn new(instances: Vec<Instance>, status: StatusMap, web_port: u16) -> Arc<Self> {
        Arc::new(Self {
            instances: Mutex::new(instances),
            status,
            web_port,
        })
    }

    /// Replace the instance list shown by the tray.
    pub fn set_instances(&self, instances: Vec<Instance>) {
        if let Ok(mut g) = self.instances.lock() {
            *g = instances;
        }
    }

    /// Snapshot for menu rendering.
    #[cfg_attr(not(windows), allow(dead_code))]
    pub(crate) fn instances_snapshot(&self) -> Vec<Instance> {
        self.instances.lock().map(|g| g.clone()).unwrap_or_default()
    }
}
