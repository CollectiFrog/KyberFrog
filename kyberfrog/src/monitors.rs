// SPDX-License-Identifier: AGPL-3.0-or-later

//! The monitors connected to *this* machine, for the viewer's "output
//! monitor" picker (#1).
//!
//! The order is kyclient's: top to bottom, then left to right (the order
//! `--display-count` fills monitors in, and the one `--output-monitor`
//! indexes). Not to be confused with [`crate::displays`], which asks a remote
//! emitter for *its* screens (the source side, #18-B).

use serde::Serialize;

/// One local monitor, in desktop coordinates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LocalMonitor {
    /// 0-based index, as `--output-monitor` takes it.
    pub index: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub primary: bool,
}

/// The connected monitors, in kyclient's order. Empty off Windows (the
/// picker then only offers the default).
pub fn list() -> Vec<LocalMonitor> {
    #[cfg(windows)]
    {
        let mut monitors = imp::enumerate();
        order(&mut monitors);
        monitors
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// Sort top to bottom, then left to right, and number them.
fn order(monitors: &mut [LocalMonitor]) {
    monitors.sort_by_key(|m| (m.y, m.x));
    for (index, monitor) in monitors.iter_mut().enumerate() {
        monitor.index = index as u32;
    }
}

#[cfg(windows)]
mod imp {
    use super::LocalMonitor;

    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{LPARAM, RECT, TRUE};
    use windows_sys::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO,
    };

    /// `MONITORINFOF_PRIMARY` (WinUser.h).
    const PRIMARY: u32 = 1;

    pub(super) fn enumerate() -> Vec<LocalMonitor> {
        let mut out: Vec<LocalMonitor> = Vec::new();
        // Safety: the callback only runs during this call and gets `out` back
        // through `lparam`, which outlives it.
        unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(callback),
                &mut out as *mut Vec<LocalMonitor> as LPARAM,
            );
        }
        out
    }

    unsafe extern "system" fn callback(
        monitor: HMONITOR,
        _hdc: HDC,
        _rect: *mut RECT,
        lparam: LPARAM,
    ) -> BOOL {
        let out = &mut *(lparam as *mut Vec<LocalMonitor>);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) != 0 {
            let r = info.rcMonitor;
            out.push(LocalMonitor {
                index: 0,
                x: r.left,
                y: r.top,
                width: r.right - r.left,
                height: r.bottom - r.top,
                primary: info.dwFlags & PRIMARY != 0,
            });
        }
        TRUE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: i32, y: i32) -> LocalMonitor {
        LocalMonitor { index: 99, x, y, width: 1920, height: 1080, primary: false }
    }

    #[test]
    fn numbered_top_to_bottom_then_left_to_right() {
        // Two side by side, one above the right-hand one.
        let mut monitors = vec![at(1920, 0), at(0, 0), at(1920, -1080)];
        order(&mut monitors);
        let seen: Vec<_> = monitors.iter().map(|m| (m.index, m.x, m.y)).collect();
        assert_eq!(seen, vec![(0, 1920, -1080), (1, 0, 0), (2, 1920, 0)]);
    }
}
