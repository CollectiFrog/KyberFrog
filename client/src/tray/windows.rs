// SPDX-License-Identifier: AGPL-3.0-or-later

//! Windows system-tray implementation for the KyberFrog Client.

use std::cell::RefCell;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use log::{error, info, warn};
use muda::{ContextMenu, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tokio::sync::mpsc;

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
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

use crate::supervisor::State;

use super::{TrayCommand, TrayModel};

const WM_TRAYICON: u32 = WM_USER + 1;
const TRAY_WINDOW_CLASS: &str = "KyberFrogClientTrayWindow";
const TOOLTIP: &str = "KyberFrog Client 🐸";

/// Menu-id separator (unit separator: safe in our ids).
const SEP: char = '\u{1f}';

/// Fixed GUID for the client tray icon (distinct from the server's).
/// {3A2C8B50-AC4F-4E21-B8F9-2E1D5C7E0A31}
const TRAY_ICON_GUID: windows_sys::core::GUID = windows_sys::core::GUID {
    data1: 0x3A2C8B50,
    data2: 0xAC4F,
    data3: 0x4E21,
    data4: [0xB8, 0xF9, 0x2E, 0x1D, 0x5C, 0x7E, 0x0A, 0x31],
};

// ---------------------------------------------------------------------------
// Handle wrappers
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct RawHandle(HANDLE);
unsafe impl Send for RawHandle {}
unsafe impl Sync for RawHandle {}

struct SendHandle(HANDLE);
unsafe impl Send for SendHandle {}
unsafe impl Sync for SendHandle {}

impl SendHandle {
    fn as_raw(&self) -> HANDLE { self.0 }
    fn is_null(&self) -> bool { self.0.is_null() }
}

impl Drop for SendHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

// ---------------------------------------------------------------------------
// Public handle
// ---------------------------------------------------------------------------

pub struct TrayHandle {
    exit_event: SendHandle,
    thread_handle: Option<JoinHandle<()>>,
}

impl TrayHandle {
    pub async fn shutdown(&mut self) {
        if !self.exit_event.is_null() {
            unsafe { SetEvent(self.exit_event.as_raw()) };
        }
        if let Some(h) = self.thread_handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for TrayHandle {
    fn drop(&mut self) {
        if !self.exit_event.is_null() {
            unsafe { SetEvent(self.exit_event.as_raw()) };
        }
        if let Some(h) = self.thread_handle.take() {
            let _ = h.join();
        }
    }
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

struct TrayContext {
    model: Arc<TrayModel>,
    menu: RefCell<Option<Menu>>,
    taskbar_created_msg: u32,
    taskbar_created: AtomicBool,
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

pub fn spawn(model: Arc<TrayModel>) -> std::io::Result<(TrayHandle, mpsc::Receiver<TrayCommand>)> {
    let (command_tx, command_rx) = mpsc::channel::<TrayCommand>(16);

    let exit_raw = unsafe { CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()) };
    if exit_raw.is_null() {
        return Err(std::io::Error::other("failed to create tray exit event"));
    }
    let exit_handle = RawHandle(exit_raw);

    let thread_handle = thread::Builder::new()
        .name("kyberfrog-client-tray".to_string())
        .spawn(move || run_tray_loop(model, command_tx, exit_handle))
        .map_err(|e| std::io::Error::other(format!("spawning tray thread: {e}")))?;

    Ok((
        TrayHandle {
            exit_event: SendHandle(exit_raw),
            thread_handle: Some(thread_handle),
        },
        command_rx,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

fn open_shell(target: &str) {
    let verb = to_wide("open");
    let wide = to_wide(target);
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
        warn!("Tray: ShellExecuteW({target}) = {}", result as isize);
    }
}

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
            info!("Tray: using custom icon {path}");
            return (raw as HICON, true);
        }
        warn!("Tray: failed to load {path}; using stock icon");
    }
    let hicon = unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) };
    (hicon, false)
}

fn custom_icon_path() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let icon = exe.parent()?.join("kyberfrog.ico");
    icon.exists().then(|| icon.to_string_lossy().into_owned())
}

// ---------------------------------------------------------------------------
// Menu
// ---------------------------------------------------------------------------

fn build_menu(model: &TrayModel) -> Menu {
    let menu = Menu::new();

    let _ = menu.append(&MenuItem::with_id("noop", TOOLTIP, false, None));
    let _ = menu.append(&PredefinedMenuItem::separator());

    let instances = model.instances_snapshot();
    let status = model.status.lock().map(|g| g.clone()).unwrap_or_default();

    if instances.is_empty() {
        let _ = menu.append(&MenuItem::with_id("noop", "(aucune instance)", false, None));
    } else {
        for inst in &instances {
            let state = status.get(&inst.id).copied().unwrap_or(State::Stopped);
            let label = format!(
                "{} {}  ·  {}:{}",
                state.symbol(),
                inst.id,
                inst.server,
                inst.port
            );
            let sub = Submenu::new(label, true);
            let _ = sub.append(&MenuItem::with_id(
                format!("start{SEP}{}", inst.id),
                "Lancer",
                true,
                None,
            ));
            let _ = sub.append(&MenuItem::with_id(
                format!("stop{SEP}{}", inst.id),
                "Stop",
                true,
                None,
            ));
            let _ = sub.append(&MenuItem::with_id(
                format!("restart{SEP}{}", inst.id),
                "Redémarrer",
                true,
                None,
            ));
            let _ = sub.append(&PredefinedMenuItem::separator());
            let _ = sub.append(&MenuItem::with_id(
                format!("remove{SEP}{}", inst.id),
                "Supprimer",
                true,
                None,
            ));
            let _ = menu.append(&sub);
        }
    }

    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("open-dashboard", "Ouvrir dashboard", true, None));
    let _ = menu.append(&MenuItem::with_id("open-logs", "Ouvrir logs", true, None));
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&MenuItem::with_id("quit", "Quitter", true, None));

    menu
}

fn parse_command(id: &MenuId) -> Option<TrayCommand> {
    let id = id.as_ref();
    if id == "quit" {
        return Some(TrayCommand::Quit);
    }
    if let Some(inst_id) = id.strip_prefix(&format!("start{SEP}")) {
        return Some(TrayCommand::Start { id: inst_id.to_string() });
    }
    if let Some(inst_id) = id.strip_prefix(&format!("stop{SEP}")) {
        return Some(TrayCommand::Stop { id: inst_id.to_string() });
    }
    if let Some(inst_id) = id.strip_prefix(&format!("restart{SEP}")) {
        return Some(TrayCommand::Restart { id: inst_id.to_string() });
    }
    if let Some(inst_id) = id.strip_prefix(&format!("remove{SEP}")) {
        return Some(TrayCommand::Remove { id: inst_id.to_string() });
    }
    None
}

// ---------------------------------------------------------------------------
// Window procedure
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tray icon
// ---------------------------------------------------------------------------

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
                CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT, CW_USEDEFAULT,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                ctx as *const std::ffi::c_void,
            )
        };
        if hwnd.is_null() {
            error!("Client tray: failed to create hidden window");
            return None;
        }

        if taskbar_created_msg != 0 {
            let ok = unsafe {
                ChangeWindowMessageFilterEx(hwnd, taskbar_created_msg, MSGFLT_ALLOW, std::ptr::null_mut())
            };
            if ok == 0 {
                warn!("Client tray: ChangeWindowMessageFilterEx failed (err {})", unsafe { GetLastError() });
            }
        }

        let (hicon, owns_icon) = load_tray_icon();
        Some(Self { hwnd, hicon, owns_icon, visible: false })
    }

    fn add(&mut self) -> bool {
        remove_orphan_icon();
        let mut nid: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = self.hwnd;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = self.hicon;
        nid.guidItem = TRAY_ICON_GUID;
        nid.uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE | NIF_GUID;
        let tip = to_wide(TOOLTIP);
        let n = tip.len().min(nid.szTip.len());
        nid.szTip[..n].copy_from_slice(&tip[..n]);
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

// ---------------------------------------------------------------------------
// Event loop
// ---------------------------------------------------------------------------

enum Wait { Exit, Message }

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
    if result == WAIT_OBJECT_0 { Wait::Exit } else { Wait::Message }
}

fn pump_messages() {
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    while unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
        unsafe { TranslateMessage(&msg); DispatchMessageW(&msg); }
    }
}

fn run_tray_loop(
    model: Arc<TrayModel>,
    command_tx: mpsc::Sender<TrayCommand>,
    exit_event: RawHandle,
) {
    info!("Client tray thread started");

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
            error!("Client tray: initialization failed");
            return;
        }
    };

    if icon.add() {
        info!("Client tray icon created");
    } else {
        warn!("Client tray icon creation failed; will retry on TaskbarCreated");
    }

    let menu_events = MenuEvent::receiver();

    loop {
        match wait_for_events(exit_event) {
            Wait::Exit => {
                info!("Client tray exit signal received");
                break;
            }
            Wait::Message => {
                pump_messages();

                if ctx.taskbar_created.swap(false, Ordering::Relaxed) {
                    info!("TaskbarCreated — re-adding client tray icon");
                    icon.add();
                }

                while let Ok(event) = menu_events.try_recv() {
                    // Handle in-thread actions directly (no AppState needed).
                    match event.id.as_ref() {
                        "open-dashboard" => {
                            let url = format!("http://localhost:{}/", ctx.model.web_port);
                            open_shell(&url);
                            continue;
                        }
                        "open-logs" => {
                            open_shell(&shared::paths::client_log_file().to_string_lossy());
                            continue;
                        }
                        _ => {}
                    }
                    if let Some(command) = parse_command(&event.id) {
                        let quit = matches!(command, TrayCommand::Quit);
                        if command_tx.blocking_send(command).is_err() {
                            warn!("Client command channel closed; exiting tray");
                            return;
                        }
                        if quit { break; }
                    }
                }
            }
        }
    }

    info!("Client tray: removing icon");
}
