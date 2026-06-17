// SPDX-License-Identifier: AGPL-3.0-or-later

//! Windows system-tray implementation for the unified KyberFrog app.
//!
//! A dedicated thread runs a `MsgWaitForMultipleObjectsEx` pump and owns the
//! `Shell_NotifyIconW` icon (with `NIF_GUID` for reliable orphan cleanup). The
//! context menu is rebuilt from the live [`TrayModel`] + a fresh Spout-sender
//! enumeration on every open, and shows both the Émission and Réception
//! sections.

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
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{CreateEventW, SetEvent};
use windows_sys::Win32::UI::Shell::{
    ShellExecuteW, Shell_NotifyIconW, NIF_GUID, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD,
    NIM_DELETE, NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    ChangeWindowMessageFilterEx, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyWindow,
    DispatchMessageW, GetWindowLongPtrW, LoadIconW, LoadImageW, MsgWaitForMultipleObjectsEx,
    PeekMessageW, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, SetWindowLongPtrW,
    TranslateMessage, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, HICON, IDI_APPLICATION,
    IMAGE_ICON, LR_DEFAULTSIZE, LR_LOADFROMFILE, MSG, MSGFLT_ALLOW, MWMO_INPUTAVAILABLE, PM_REMOVE,
    QS_ALLINPUT, SW_SHOWNORMAL, WM_LBUTTONUP, WM_NCCREATE, WM_RBUTTONUP, WM_USER, WNDCLASSW,
    WS_OVERLAPPEDWINDOW,
};

use crate::supervisor::{state_of, Key};

use super::{TrayCommand, TrayModel};

/// Custom message for tray icon callbacks.
const WM_TRAYICON: u32 = WM_USER + 1;
/// Hidden window class name.
const TRAY_WINDOW_CLASS: &str = "KyberFrogTrayWindow";
/// Tooltip text (also the disabled menu header).
const TOOLTIP: &str = "KyberFrog 🐸";

/// Menu-id separator (unit separator: cannot appear in names/ids in practice,
/// and never in our own action prefixes).
const SEP: char = '\u{1f}';

/// Fixed GUID identifying our tray icon across restarts.
/// {4B7E2C10-5A9D-4F31-9C2E-7D0A6B1F3E54}
const TRAY_ICON_GUID: windows_sys::core::GUID = windows_sys::core::GUID {
    data1: 0x4B7E2C10,
    data2: 0x5A9D,
    data3: 0x4F31,
    data4: [0x9C, 0x2E, 0x7D, 0x0A, 0x6B, 0x1F, 0x3E, 0x54],
};

// ---------------------------------------------------------------------------
// Send-safe handle wrappers
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

/// Handle the app uses to stop the tray thread.
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
        .name("kyberfrog-tray".to_string())
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

/// Open `target` (a file path or a URL) with its default application.
fn open_shell(target: &str) {
    let verb = to_wide("open");
    let wide = to_wide(target);
    // ShellExecuteW returns a value > 32 on success.
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            wide.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL as i32,
        )
    };
    if (result as isize) <= 32 {
        warn!("Tray: failed to open {target} (ShellExecuteW = {})", result as isize);
    }
}

/// Load the tray icon: file override next to the exe, then embedded resource,
/// then the stock application icon. The bool reports caller ownership.
fn load_tray_icon() -> (HICON, bool) {
    if let Some(path) = custom_icon_path() {
        let wide = to_wide(&path);
        let raw = unsafe {
            LoadImageW(
                std::ptr::null_mut(),
                wide.as_ptr(),
                IMAGE_ICON,
                0,
                0,
                LR_LOADFROMFILE | LR_DEFAULTSIZE,
            )
        };
        if !raw.is_null() {
            info!("Tray: using icon file {path}");
            return (raw as HICON, true);
        }
    }

    let hinstance = unsafe { GetModuleHandleW(std::ptr::null()) };
    if !hinstance.is_null() {
        let raw = unsafe {
            LoadImageW(hinstance, 1usize as *const u16, IMAGE_ICON, 0, 0, LR_DEFAULTSIZE)
        };
        if !raw.is_null() {
            info!("Tray: using embedded icon");
            return (raw as HICON, true);
        }
    }

    warn!("Tray: falling back to stock IDI_APPLICATION icon");
    let hicon = unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) };
    (hicon, false)
}

/// Path to a `kyberfrog.ico` sitting next to the executable, if it exists.
fn custom_icon_path() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let icon = exe.parent()?.join("kyberfrog.ico");
    icon.exists().then(|| icon.to_string_lossy().into_owned())
}

/// Build the context menu from the current model + live Spout senders.
fn build_menu(model: &TrayModel) -> Menu {
    let menu = Menu::new();

    let _ = menu.append(&MenuItem::with_id("noop", TOOLTIP, false, None));
    let _ = menu.append(&PredefinedMenuItem::separator());

    let status = model.status.lock().map(|g| g.clone()).unwrap_or_default();

    // -- Émission ----------------------------------------------------------
    let _ = menu.append(&MenuItem::with_id("noop", "— Émission —", false, None));
    let transmitters = model.transmitters_snapshot();
    if transmitters.is_empty() {
        let _ = menu.append(&MenuItem::with_id("noop", "(aucun transmetteur)", false, None));
    } else {
        for tx in &transmitters {
            let state = state_of(&status, &Key::Tx(tx.name.clone()));
            let label = format!(
                "{} {}  :{}  ·  {}",
                state.symbol(),
                tx.name,
                tx.port,
                tx.source.label()
            );
            let submenu = Submenu::new(label, true);
            let _ = submenu.append(&MenuItem::with_id(
                format!("tx-restart{SEP}{}", tx.name),
                "Redémarrer",
                true,
                None,
            ));
            let _ = submenu.append(&MenuItem::with_id(
                format!("tx-remove{SEP}{}", tx.name),
                "Supprimer",
                true,
                None,
            ));
            let _ = menu.append(&submenu);
        }
    }

    // "Add transmitter" with a live Spout sender list.
    let add = Submenu::new("Ajouter un transmetteur", true);
    let senders = crate::spout::list_senders();
    if senders.names.is_empty() {
        let _ = add.append(&MenuItem::with_id(
            "noop",
            "(aucun sender Spout détecté)",
            false,
            None,
        ));
    } else {
        for sender in &senders.names {
            let active = senders.active.as_deref() == Some(sender.as_str());
            let label = if active {
                format!("Spout: {sender}  (actif)")
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
    let _ = add.append(&MenuItem::with_id("add-screen", "Capture d'écran", true, None));
    let _ = menu.append(&add);

    let _ = menu.append(&PredefinedMenuItem::separator());

    // -- Réception ---------------------------------------------------------
    let _ = menu.append(&MenuItem::with_id("noop", "— Réception —", false, None));
    let viewers = model.viewers_snapshot();
    if viewers.is_empty() {
        let _ = menu.append(&MenuItem::with_id("noop", "(aucune réception)", false, None));
    } else {
        for v in &viewers {
            let state = state_of(&status, &Key::Vw(v.id.clone()));
            let label = format!("{} {}  ·  {}:{}", state.symbol(), v.id, v.server, v.port);
            let submenu = Submenu::new(label, true);
            let _ = submenu.append(&MenuItem::with_id(
                format!("vw-start{SEP}{}", v.id),
                "Lancer",
                true,
                None,
            ));
            let _ = submenu.append(&MenuItem::with_id(
                format!("vw-stop{SEP}{}", v.id),
                "Stop",
                true,
                None,
            ));
            let _ = submenu.append(&MenuItem::with_id(
                format!("vw-restart{SEP}{}", v.id),
                "Redémarrer",
                true,
                None,
            ));
            let _ = submenu.append(&PredefinedMenuItem::separator());
            let _ = submenu.append(&MenuItem::with_id(
                format!("vw-remove{SEP}{}", v.id),
                "Supprimer",
                true,
                None,
            ));
            let _ = menu.append(&submenu);
        }
    }

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("open-dashboard", "Ouvrir dashboard", true, None));
    let _ = menu.append(&MenuItem::with_id("open-config", "Ouvrir config", true, None));
    let _ = menu.append(&MenuItem::with_id("open-logs", "Ouvrir logs", true, None));

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("quit", "Quitter", true, None));

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
        return Some(TrayCommand::AddSpout { sender: sender.to_string() });
    }
    if let Some(name) = id.strip_prefix(&format!("tx-restart{SEP}")) {
        return Some(TrayCommand::RestartTx { name: name.to_string() });
    }
    if let Some(name) = id.strip_prefix(&format!("tx-remove{SEP}")) {
        return Some(TrayCommand::RemoveTx { name: name.to_string() });
    }
    if let Some(vid) = id.strip_prefix(&format!("vw-start{SEP}")) {
        return Some(TrayCommand::StartViewer { id: vid.to_string() });
    }
    if let Some(vid) = id.strip_prefix(&format!("vw-stop{SEP}")) {
        return Some(TrayCommand::StopViewer { id: vid.to_string() });
    }
    if let Some(vid) = id.strip_prefix(&format!("vw-restart{SEP}")) {
        return Some(TrayCommand::RestartViewer { id: vid.to_string() });
    }
    if let Some(vid) = id.strip_prefix(&format!("vw-remove{SEP}")) {
        return Some(TrayCommand::RemoveViewer { id: vid.to_string() });
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
    /// `true` if `hicon` was loaded from file and must be `DestroyIcon`-ed.
    owns_icon: bool,
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

        let (hicon, owns_icon) = load_tray_icon();

        Some(Self {
            hwnd,
            hicon,
            owns_icon,
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
        if self.owns_icon && !self.hicon.is_null() {
            unsafe { DestroyIcon(self.hicon) };
        }
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

fn run_tray_loop(
    model: Arc<TrayModel>,
    command_tx: mpsc::Sender<TrayCommand>,
    exit_event: RawHandle,
) {
    info!("Tray thread started");

    let taskbar_created_msg =
        unsafe { RegisterWindowMessageW(to_wide("TaskbarCreated").as_ptr()) };

    let web_port = model.web_port;
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
                    // "Open …" items are handled here directly (no app state
                    // needed); everything else becomes a command.
                    match event.id.as_ref() {
                        "open-dashboard" => {
                            open_shell(&format!("http://localhost:{web_port}/"));
                            continue;
                        }
                        "open-config" => {
                            open_shell(&shared::paths::config_file().to_string_lossy());
                            continue;
                        }
                        "open-logs" => {
                            open_shell(&shared::paths::app_log_file().to_string_lossy());
                            continue;
                        }
                        _ => {}
                    }
                    if let Some(command) = parse_command(&event.id) {
                        let quit = matches!(command, TrayCommand::Quit);
                        if command_tx.blocking_send(command).is_err() {
                            warn!("App command channel closed; exiting tray");
                            return;
                        }
                        if quit {
                            // The app will signal the exit event; keep the icon
                            // up until then so the user gets feedback.
                        }
                    }
                }
            }
        }
    }

    info!("Removing tray icon");
}
