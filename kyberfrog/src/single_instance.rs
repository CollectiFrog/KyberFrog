// SPDX-License-Identifier: AGPL-3.0-or-later

//! One KyberFrog per user session.
//!
//! The app is not built to run twice: a second instance would start its own
//! transmitters and viewers next to the first one's (kycontrollers crash-looping
//! on taken ports, duplicate kyclients), and on Windows it would steal the
//! first one's tray icon (same fixed GUID). So this check runs **first**, before
//! anything is started; a second launch hands over to the running instance —
//! on Windows it asks it to show its dashboard window — and exits.

/// Outcome of [`acquire`].
pub enum Instance {
    /// We are the only instance; the lock is held until the process exits.
    Primary,
    /// Another instance already runs (and was asked to show its dashboard).
    Secondary,
}

/// Take the per-session instance lock, or hand over to the instance holding it.
pub fn acquire() -> Instance {
    imp::acquire()
}

#[cfg(windows)]
mod imp {
    use log::{info, warn};
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
    use windows_sys::Win32::System::Threading::CreateMutexW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AllowSetForegroundWindow, FindWindowExW, GetWindowThreadProcessId, PostMessageW,
        RegisterWindowMessageW,
    };

    use super::Instance;
    use crate::tray::{SHOW_DASHBOARD_MSG, TRAY_WINDOW_CLASS};

    /// `Local\`: one instance per logon session, as the tray and the window are.
    const MUTEX_NAME: &str = "Local\\KyberFrog.SingleInstance";

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn acquire() -> Instance {
        // Never closed: the OS releases the mutex when the process exits.
        let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, wide(MUTEX_NAME).as_ptr()) };
        let err = unsafe { GetLastError() };
        if mutex.is_null() {
            warn!("Single-instance mutex unavailable (err {err}); not enforcing one instance");
            return Instance::Primary;
        }
        if err != ERROR_ALREADY_EXISTS {
            return Instance::Primary;
        }

        info!("KyberFrog is already running: showing its dashboard and exiting");
        let class = wide(TRAY_WINDOW_CLASS);
        let msg = unsafe { RegisterWindowMessageW(wide(SHOW_DASHBOARD_MSG).as_ptr()) };
        let mut signalled = 0;
        let mut hwnd = std::ptr::null_mut();
        // Every tray window of our class: a pre-0.8.1 KyberFrog (no mutex)
        // may run next to the one holding it, and simply ignores the message.
        loop {
            hwnd = unsafe {
                FindWindowExW(std::ptr::null_mut(), hwnd, class.as_ptr(), std::ptr::null())
            };
            if hwnd.is_null() || msg == 0 {
                break;
            }
            unsafe {
                // We were just launched by the user, so we may hand our
                // foreground right over: the instance can then raise its window.
                let mut pid = 0;
                GetWindowThreadProcessId(hwnd, &mut pid);
                if pid != 0 {
                    AllowSetForegroundWindow(pid);
                }
                PostMessageW(hwnd, msg, 0, 0);
            }
            signalled += 1;
        }
        if signalled == 0 {
            // Still starting, or running without a tray: nothing to signal.
            warn!("Running instance has no tray window to signal");
        }
        Instance::Secondary
    }
}

#[cfg(unix)]
mod imp {
    use std::os::fd::AsRawFd;

    use log::{info, warn};

    use super::Instance;

    pub fn acquire() -> Instance {
        let path = shared::paths::app_data_dir().join("kyberfrog.lock");
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let file = match std::fs::OpenOptions::new().create(true).write(true).open(&path) {
            Ok(file) => file,
            Err(err) => {
                warn!("Cannot open {path:?} ({err}); not enforcing one instance");
                return Instance::Primary;
            }
        };
        // flock is released by the kernel when the process exits, crash included.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            std::mem::forget(file); // hold the lock for the whole process lifetime
            return Instance::Primary;
        }
        info!("KyberFrog is already running for this user ({path:?} is locked): exiting");
        Instance::Secondary
    }
}
