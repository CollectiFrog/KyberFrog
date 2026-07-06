// SPDX-License-Identifier: AGPL-3.0-or-later

//! Query a remote emitter for its list of physical displays.
//!
//! Feeds the viewer form's "source screen" picker: the operator connects a
//! viewer to an emitter (`server:port`) and picks *which* of that emitter's
//! monitors to stream. Display selection is a client-side decision in Kyber
//! (kyclient sends a `display_id` at stream start; the emitter serves whatever
//! is asked), so KyberFrog surfaces it on the viewer as `display_idx` — a
//! 0-based index into the list this module returns.
//!
//! The emitter is a `kycontroller` serving its control API over **HTTPS with a
//! self-signed (TOFU) certificate** on the control-plane port. `GET
//! /enumerate_displays` returns the cached list (populated at controller
//! startup) and needs no session — the Cross-Origin Protection middleware lets
//! safe methods (GET) through unconditionally — so we deliberately do **not**
//! log in: a login would open a session and could evict a live viewer under
//! kycontroller's single-session-per-instance policy.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// One display as returned to the UI. A subset of kycontroller's `Display`
/// (its `x`/`y` offsets are not useful for the picker).
#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfo {
    /// The emitter's own display id (informational; the viewer selects by
    /// list *index*, not by this id — see the module docs).
    pub id: u32,
    pub name: String,
    pub width: i32,
    pub height: i32,
}

/// kycontroller's `Display` wire shape (`kyavservice_types::Display`).
#[derive(Deserialize)]
struct RawDisplay {
    id: u32,
    name: String,
    width: i32,
    height: i32,
    // `x`, `y` are present on the wire but unused here.
}

/// Enumerate the displays of the emitter at `server:port`. Returns them in the
/// controller's order — the same order a viewer's `display_idx` indexes into.
pub async fn enumerate(server: &str, port: u16) -> anyhow::Result<Vec<DisplayInfo>> {
    let url = format!("https://{server}:{port}/enumerate_displays");

    // TOFU self-signed cert on a trusted LAN: accept it (mirrors the viewer's
    // own `--tls-tofu`). native-tls → SChannel on the Windows target, so no
    // OpenSSL/rustls provider is pulled into the cross-build.
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(4))
        .build()?;

    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        anyhow::bail!("emitter returned {} for {url}", response.status());
    }

    let raw: Vec<RawDisplay> = response.json().await?;
    Ok(raw
        .into_iter()
        .map(|d| DisplayInfo {
            id: d.id,
            name: d.name,
            width: d.width,
            height: d.height,
        })
        .collect())
}
