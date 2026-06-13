// SPDX-License-Identifier: AGPL-3.0-or-later

//! Windows system-tray implementation for the Director.
//!
//! Structure mirrors Kyber's `kycontroller` tray (a dedicated thread with a
//! `MsgWaitForMultipleObjectsEx` pump, `Shell_NotifyIconW` with `NIF_GUID` for
//! reliable orphan cleanup), with two differences:
//!
//! * the context menu is rebuilt from the live [`TrayModel`] + Spout senders on
//!   every open, and
//! * a stock application icon is used (no PNG asset / GDI decoding).

use std::cell::RefCell;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use log::{error, info, warn};
use muda::{ContextMenu, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tokio::sync::mpsc;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows_sys::Win32::System::Threading::{CreateEventW, SetEvent};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_GUID, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE,
    NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    ChangeWindowMessageFilterEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
    GetWindowLongPtrW, LoadIconW, MsgWaitForMultipleObjectsEx, PeekMessageW, RegisterClassW,
    RegisterWindowMessageW, SetForegroundWindow, SetWindowLongPtrW, TranslateMessage, CREATESTRUCTW,
    CW_USEDEFAULT, GWLP_USERDATA, HICON, IDI_APPLICATION, MSG, MSGFLT_ALLOW, MWMO_INPUTAVAILABLE,
    PM_REMOVE, QS_ALLINPUT, WM_LBUTTONUP, WM_NCCREATE, WM_RBUTTONUP, WM_USER, WNDCLASSW,
    WS_OVERLAPPEDWINDOW,
};

use crate::supervisor::State;

use super::{TrayCommand, TrayModel};

/// Custom message for tray icon callbacks.
const WM_TRAYICON: u32 = WM_USER + 1;
/// Hidden window class name.
const TRAY_WINDOW_CLASS: &str = "KyberAnySourceTrayWindow";
/// Tooltip text.
const TOOLTIP: &str = "kyber-anysource Director";

/// Menu-id separator (unit separator: cannot appear in sender names typed by
/// users in practice, and never in our own action prefixes).
const SEP: char = '\u{1f}';

/// Fixed GUID identifying our tray icon across restarts (distinct from
/// kycontroller's). {2F1C7A40-9B3E-4D21-A6F8-1E0C5B7D9A42}
const TRAY_ICON_GUID: windows_sys::core::GUID = windows_sys::core::GUID {
    data1: 0x2F1C7A40,
    data2: 0x9B3E,
    data3: 0x4D21,
    data4: [0xA6, 0xF8, 0x1E, 0x0C, 0x5B, 0x7D, 0x9A, 0x42],
};

// ---------------------------------------------------------------------------
// Send-safe handle wrappers (Windows event handles are ref-counted kernel
// objects, safe to move across threads).
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct RawHandle(HANDLE);
unsafe impl Send for RawHandle {}
unsafe impl Sync for RawHandle {}

struct SendHandle(HANDLE);
unsafe impl Send for SendHandle {}
unsafe impl Sync for SendHandle {}

impl SendHandle {
    fn as_raw(&self) -> HANDLE {
        self.0
    }
    fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl Drop for SendHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// Handle the Director uses to stop the tray thread.
pub struct TrayHandle {
    exit_event: SendHandle,
    thread_handle: Option<JoinHandle<()>>,
}

impl TrayHandle {
    /// Signal the tray thread to exit and wait for it.
    pub async fn shutdown(&mut self) {
        if !self.exit_event.is_null() {
            unsafe { SetEvent(self.exit_event.as_raw()) };
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for TrayHandle {
    fn drop(&mut self) {
        if !self.exit_event.is_null() {
            unsafe { SetEvent(self.exit_event.as_raw()) };
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Per-window context stored in the hidden window's user data.
struct TrayContext {
    model: Arc<TrayModel>,
    /// Menu kept alive across the modal `TrackPopupMenu` call.
    menu: RefCell<Option<Menu>>,
    taskbar_created_msg: u32,
    taskbar_created: AtomicBool,
}

/// Spawn the tray thread. Returns a handle and the command receiver.
pub fn spawn(model: Arc<TrayModel>) -> std::io::Result<(TrayHandle, mpsc::Receiver<TrayCommand>)> {
    let (command_tx, command_rx) = mpsc::channel::<TrayCommand>(16);

    // Manual-reset exit event (stays signaled once set).
    let exit_raw = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
    if exit_raw.is_null() {
        return Err(std::io::Error::other("failed to create tray exit event"));
    }
    let exit_handle = RawHandle(exit_raw);

    let thread_handle = thread::Builder::new()
        .name("director-tray".to_string())
        .spawn(move || {
            run_tray_loop(model, command_tx, exit_handle);
        })
        .map_err(|e| std::io::Error::other(format!("spawning tray thread: {e}")))?;

    Ok((
        TrayHandle {
            exit_event: SendHandle(exit_raw),
            thread_handle: Some(thread_handle),
        },
        command_rx,
    ))
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Build the context menu from the current model + live Spout senders.
fn build_menu(model: &TrayModel) -> Menu {
    let menu = Menu::new();

    let header = MenuItem::with_id("noop", TOOLTIP, false, None);
    let _ = menu.append(&header);
    let _ = menu.append(&PredefinedMenuItem::separator());

    let transmitters = model.transmitters_snapshot();
    let status = model.status.lock().map(|g| g.clone()).unwrap_or_default();

    if transmitters.is_empty() {
        let _ = menu.append(&MenuItem::with_id("noop", "(no transmitters)", false, None));
    } else {
        for tx in &transmitters {
            let state = status.get(&tx.name).copied().unwrap_or(State::Stopped);
            let label = format!(
                "{} {}  :{}  ·  {}",
                state.symbol(),
                tx.name,
                tx.port,
                tx.source.label()
            );
            let submenu = Submenu::new(label, true);
            let _ = submenu.append(&MenuItem::with_id(
                format!("restart{SEP}{}", tx.name),
                "Restart",
                true,
                None,
            ));
            let _ = submenu.append(&MenuItem::with_id(
                format!("remove{SEP}{}", tx.name),
                "Remove",
                true,
                None,
            ));
            let _ = menu.append(&submenu);
        }
    }

    let _ = menu.append(&PredefinedMenuItem::separator());

    // "Add transmitter" with a live Spout sender list.
    let add = Submenu::new("Add transmitter", true);
    let senders = crate::spout::list_senders();
    if senders.names.is_empty() {
        let _ = add.append(&MenuItem::with_id(
            "noop",
            "(no Spout sender detected)",
            false,
            None,
        ));
    } else {
        for sender in &senders.names {
            let active = senders.active.as_deref() == Some(sender.as_str());
            let label = if active {
                format!("Spout: {sender}  (active)")
            } else {
                format!("Spout: {sender}")
            };
            let _ = add.append(&MenuItem::with_id(
                format!("add-spout{SEP}{sender}"),
                label,
                true,
                None,
            ));
        }
    }
    let _ = add.append(&PredefinedMenuItem::separator());
    let _ = add.append(&MenuItem::with_id("add-screen", "Screen capture", true, None));
    let _ = menu.append(&add);

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("quit", "Quit", true, None));

    menu
}

/// Translate a menu id into a [`TrayCommand`].
fn parse_command(id: &MenuId) -> Option<TrayCommand> {
    let id = id.as_ref();
    if id == "quit" {
        return Some(TrayCommand::Quit);
    }
    if id == "add-screen" {
        return Some(TrayCommand::AddScreen);
    }
    if let Some(sender) = id.strip_prefix(&format!("add-spout{SEP}")) {
        return Some(TrayCommand::AddSpout {
            sender: sender.to_string(),
        });
    }
    if let Some(name) = id.strip_prefix(&format!("remove{SEP}")) {
        return Some(TrayCommand::Remove {
            name: name.to_string(),
        });
    }
    if let Some(name) = id.strip_prefix(&format!("restart{SEP}")) {
        return Some(TrayCommand::Restart {
            name: name.to_string(),
        });
    }
    None
}

/// Window procedure: shows the freshly-built menu on click; tracks
/// TaskbarCreated so the icon can be re-added after Explorer restarts.
unsafe extern "system" fn tray_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(lparam as *const CREATESTRUCTW);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }

    let userdata = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
    if userdata == 0 {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let ctx = &*(userdata as *const TrayContext);

    if ctx.taskbar_created_msg != 0 && msg == ctx.taskbar_created_msg {
        ctx.taskbar_created.store(true, Ordering::Relaxed);
        return 0;
    }

    if msg == WM_TRAYICON {
        let event = (lparam & 0xFFFF) as u32;
        if event == WM_RBUTTONUP || event == WM_LBUTTONUP {
            SetForegroundWindow(hwnd);
            // Rebuild the menu from current state, keep it alive for the modal
            // TrackPopupMenu, then show it at the cursor.
            let menu = build_menu(&ctx.model);
            menu.show_context_menu_for_hwnd(hwnd as isize, None);
            *ctx.menu.borrow_mut() = Some(menu);
        }
        return 0;
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

fn create_notify_icon_data(hwnd: HWND, hicon: HICON) -> NOTIFYICONDATAW {
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 0; // Unused with NIF_GUID.
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = hicon;
    nid.guidItem = TRAY_ICON_GUID;

    let tip = to_wide(TOOLTIP);
    let n = tip.len().min(nid.szTip.len());
    nid.szTip[..n].copy_from_slice(&tip[..n]);
    nid
}

/// Remove any leftover icon with our GUID (orphan from a crashed instance).
fn remove_orphan_icon() {
    let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.uFlags = NIF_GUID;
    nid.guidItem = TRAY_ICON_GUID;
    unsafe { Shell_NotifyIconW(NIM_DELETE, &nid) };
}

struct TrayIcon {
    hwnd: HWND,
    hicon: HICON,
    visible: bool,
}

impl TrayIcon {
    fn new(ctx: *const TrayContext, taskbar_created_msg: u32) -> Option<Self> {
        let class_name = to_wide(TRAY_WINDOW_CLASS);
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(tray_window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: std::ptr::null_mut(),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        unsafe { RegisterClassW(&wc) };

        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                std::ptr::null(),
                WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                ctx as *const std::ffi::c_void,
            )
        };
        if hwnd.is_null() {
            error!("Tray: failed to create hidden window");
            return None;
        }

        if taskbar_created_msg != 0 {
            let ok = unsafe {
                ChangeWindowMessageFilterEx(
                    hwnd,
                    taskbar_created_msg,
                    MSGFLT_ALLOW,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 {
                warn!("Tray: ChangeWindowMessageFilterEx failed (err {})", unsafe {
                    GetLastError()
                });
            }
        }

        // Stock application icon (no asset / GDI work needed).
        let hicon = unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) };

        Some(Self {
            hwnd,
            hicon,
            visible: false,
        })
    }

    fn add(&mut self) -> bool {
        remove_orphan_icon();
        let mut nid = create_notify_icon_data(self.hwnd, self.hicon);
        nid.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE | NIF_GUID;
        let ok = unsafe { Shell_NotifyIconW(NIM_ADD, &nid) };
        self.visible = ok != 0;
        self.visible
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        if self.visible {
            let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.uFlags = NIF_GUID;
            nid.guidItem = TRAY_ICON_GUID;
            unsafe { Shell_NotifyIconW(NIM_DELETE, &nid) };
        }
        if !self.hwnd.is_null() {
            unsafe { DestroyWindow(self.hwnd) };
        }
        // hicon is a shared stock icon: do not destroy.
    }
}

enum Wait {
    Exit,
    Message,
}

fn wait_for_events(exit_event: RawHandle) -> Wait {
    use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
    use windows_sys::Win32::System::Threading::INFINITE;

    let handles = [exit_event.0];
    let result = unsafe {
        MsgWaitForMultipleObjectsEx(
            handles.len() as u32,
            handles.as_ptr(),
            INFINITE,
            QS_ALLINPUT,
            MWMO_INPUTAVAILABLE,
        )
    };
    if result == WAIT_OBJECT_0 {
        Wait::Exit
    } else {
        Wait::Message
    }
}

fn pump_messages() {
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    while unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn run_tray_loop(model: Arc<TrayModel>, command_tx: mpsc::Sender<TrayCommand>, exit_event: RawHandle) {
    info!("Tray thread started");

    let taskbar_created_msg =
        unsafe { RegisterWindowMessageW(to_wide("TaskbarCreated").as_ptr()) };

    let ctx = TrayContext {
        model,
        menu: RefCell::new(None),
        taskbar_created_msg,
        taskbar_created: AtomicBool::new(false),
    };

    let mut icon = match TrayIcon::new(&ctx, taskbar_created_msg) {
        Some(icon) => icon,
        None => {
            error!("Tray: initialization failed; tray disabled");
            return;
        }
    };

    if icon.add() {
        info!("System tray icon created");
    } else {
        warn!("Tray icon creation failed; will retry on TaskbarCreated");
    }

    let menu_events = MenuEvent::receiver();

    loop {
        match wait_for_events(exit_event) {
            Wait::Exit => {
                info!("Tray exit signal received");
                break;
            }
            Wait::Message => {
                pump_messages();

                if ctx.taskbar_created.swap(false, Ordering::Relaxed) {
                    info!("TaskbarCreated received, re-adding tray icon");
                    icon.add();
                }

                while let Ok(event) = menu_events.try_recv() {
                    if let Some(command) = parse_command(&event.id) {
                        let quit = matches!(command, TrayCommand::Quit);
                        if command_tx.blocking_send(command).is_err() {
                            warn!("Director command channel closed; exiting tray");
                            return;
                        }
                        if quit {
                            // The Director will signal the exit event; keep the
                            // icon up until then so the user gets feedback.
                        }
                    }
                }
            }
        }
    }

    info!("Removing tray icon");
    // `icon` and `ctx` drop here (icon removed via Shell_NotifyIconW).
}
