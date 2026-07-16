// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tauri/WebView2 shell — the Windows implementation of [`super::run`].
//!
//! The Tauri (tao) event loop must own the **main thread**; the whole async
//! app (supervisor, web server, mDNS, tray) keeps running on the tokio runtime
//! built by `main`, and the two sides meet through the tray-command loop
//! spawned here. The window is pure chrome: it loads the same
//! `http://localhost:<web_port>/` the browser would, so `web.rs` and the React
//! app are untouched (same-origin, no IPC).

use log::{error, info, warn};
use tauri::webview::DownloadEvent;
use tauri::{Manager, RunEvent, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use super::{dispatch, shutdown, Boot, Flow};

/// Label of the (single) dashboard window.
const MAIN_WINDOW: &str = "main";

pub fn run(runtime: tokio::runtime::Runtime, boot: Boot) -> anyhow::Result<()> {
    let Boot {
        state,
        web_task,
        mut tray_handle,
        mut command_rx,
        web_port,
    } = boot;

    // Share our runtime with tauri's internals instead of letting it spawn a
    // second one.
    tauri::async_runtime::set(runtime.handle().clone());

    let app = tauri::Builder::default()
        .setup(move |app| {
            create_main_window(app.handle(), web_port)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window never quits the app: hide it and let the tray
            // keep running (only its "Quitter" stops the process).
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())?;

    // The old main loop, now a task: apply tray commands, and on quit run the
    // shared shutdown sequence before ending the (main-thread) event loop.
    let handle = app.handle().clone();
    runtime.spawn(async move {
        info!("KyberFrog ready");
        loop {
            tokio::select! {
                maybe = command_rx.recv() => {
                    let Some(command) = maybe else {
                        // Tray gone: the window and the browser UI still work;
                        // idle until Ctrl-C.
                        let _ = tokio::signal::ctrl_c().await;
                        break;
                    };
                    match dispatch(command, &state).await {
                        Flow::Continue => {}
                        Flow::OpenDashboard => show_main_window(&handle, web_port),
                        Flow::Quit => break,
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    info!("Ctrl-C received");
                    break;
                }
            }
        }
        shutdown(&state, &web_task, &mut tray_handle).await;
        handle.exit(0);
    });

    app.run(|_handle, event| {
        if let RunEvent::ExitRequested { api, code, .. } = &event {
            // Only our own `exit(0)` above — issued *after* the shutdown
            // sequence — may end the process; a window-driven exit (no code)
            // is vetoed, mirroring the close-is-hide policy.
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });

    info!("KyberFrog stopped");
    runtime.shutdown_timeout(std::time::Duration::from_secs(5));
    Ok(())
}

/// URL the window points at: the embedded axum server, or the vite dev server
/// (HMR) when `KYBERFROG_UI_URL` is set (e.g. `http://localhost:5173/`).
fn dashboard_url(web_port: u16) -> tauri::Url {
    let default = format!("http://localhost:{web_port}/");
    let raw = std::env::var("KYBERFROG_UI_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default.clone());
    tauri::Url::parse(&raw).unwrap_or_else(|err| {
        warn!("Invalid KYBERFROG_UI_URL {raw:?} ({err}); using {default}");
        tauri::Url::parse(&default).expect("default dashboard URL is valid")
    })
}

fn create_main_window(
    handle: &tauri::AppHandle,
    web_port: u16,
) -> tauri::Result<tauri::WebviewWindow> {
    let url = dashboard_url(web_port);
    info!("Opening dashboard window on {url}");
    WebviewWindowBuilder::new(handle, MAIN_WINDOW, WebviewUrl::External(url))
        .title("KyberFrog")
        .inner_size(1280.0, 840.0)
        .min_inner_size(880.0, 560.0)
        // The window has no browser download bar: without this handler a
        // download (« télécharger une config ») lands silently in
        // ~\Downloads and the operator sees nothing happen. Keep WebView2's
        // default destination, then reveal the file in Explorer when done.
        .on_download(|_webview, event| {
            match event {
                DownloadEvent::Requested { url, destination } => {
                    info!("Downloading {url} to {}", destination.display());
                }
                DownloadEvent::Finished { url, path, success } => {
                    if !success {
                        warn!("Download of {url} failed");
                    } else if let Some(path) = path {
                        info!("Downloaded {url} to {}", path.display());
                        let _ = std::process::Command::new("explorer")
                            .arg(format!("/select,{}", path.display()))
                            .spawn();
                    }
                }
                _ => {}
            }
            true // always let the download proceed
        })
        .build()
}

/// Tray "Ouvrir dashboard": surface the (possibly hidden) window, recreating
/// it if it is gone; fall back to the browser if the webview itself fails
/// (e.g. WebView2 missing) so the operator is never locked out.
fn show_main_window(handle: &tauri::AppHandle, web_port: u16) {
    match handle.get_webview_window(MAIN_WINDOW) {
        Some(window) => {
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
        None => {
            if let Err(err) = create_main_window(handle, web_port) {
                error!("Cannot open the dashboard window: {err}; opening the browser instead");
                crate::tray::open_shell(&format!("http://localhost:{web_port}/"));
            }
        }
    }
}
