// SPDX-License-Identifier: AGPL-3.0-or-later

//! Headless shell for non-Windows targets — exactly the pre-#21 run loop, so
//! the app keeps building and running for cross-platform `cargo check`/tests.

use log::info;

use super::{dispatch, shutdown, Boot, Flow};

pub fn run(runtime: tokio::runtime::Runtime, boot: Boot) -> anyhow::Result<()> {
    let Boot {
        state,
        web_task,
        mut tray_handle,
        mut command_rx,
        web_port,
    } = boot;

    runtime.block_on(async move {
        info!("KyberFrog ready (headless: dashboard at http://localhost:{web_port}/)");
        loop {
            tokio::select! {
                maybe = command_rx.recv() => {
                    let Some(command) = maybe else {
                        // Tray gone: idle until Ctrl-C.
                        let _ = tokio::signal::ctrl_c().await;
                        break;
                    };
                    match dispatch(command, &state).await {
                        Flow::Continue => {}
                        // No window to show; the browser UI is always up.
                        Flow::OpenDashboard => {}
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
    });

    info!("KyberFrog stopped");
    Ok(())
}
