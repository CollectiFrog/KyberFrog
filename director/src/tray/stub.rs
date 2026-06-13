// SPDX-License-Identifier: AGPL-3.0-or-later

//! No-op tray for non-Windows targets (keeps the Director buildable and
//! runnable for cross-platform `cargo check`/tests). The command receiver never
//! yields, so the Director simply runs headless until Ctrl-C.

use std::sync::Arc;

use tokio::sync::mpsc;

use super::{TrayCommand, TrayModel};

pub struct TrayHandle;

impl TrayHandle {
    pub async fn shutdown(&mut self) {}
}

pub fn spawn(_model: Arc<TrayModel>) -> std::io::Result<(TrayHandle, mpsc::Receiver<TrayCommand>)> {
    log::warn!("System tray is only available on Windows; running headless");
    let (_tx, rx) = mpsc::channel(1);
    Ok((TrayHandle, rx))
}
