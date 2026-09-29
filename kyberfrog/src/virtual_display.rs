// SPDX-License-Identifier: AGPL-3.0-or-later

//! The virtual screen of a screen-capture transmitter on a machine with no
//! monitor plugged in (#54), through the Virtual Display Driver (VDD,
//! VirtualDrivers, MIT, signed).
//!
//! Only installing the driver needs admin rights (the installer's optional
//! section). Everything here runs as the user:
//!
//! - the installer writes the driver's modes once, in
//!   `C:\VirtualDisplayDriver\vdd_settings.xml`: the four sizes the form
//!   offers, at 60 Hz (`packaging/windows/vdd_settings.xml`). KyberFrog never
//!   reloads the driver: `RELOAD_DRIVER` on its pipe, sent while the screen
//!   was detached, left it crashed (code 43) until an admin restarted it;
//! - the size is picked with `ChangeDisplaySettingsEx`, which also attaches
//!   the screen to the desktop; releasing it detaches it (width 0).
//!
//! The driver always keeps one monitor: "no virtual screen" is a detached
//! one, invisible to the desktop and to the capture.

use anyhow::Result;
use serde::Serialize;
use shared::VirtualDisplay;

/// What the web UI needs to offer the option.
#[derive(Clone, Debug, Serialize)]
pub struct Availability {
    /// The driver is installed and running.
    pub available: bool,
    /// Why not, in French, for the form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

pub fn availability() -> Availability {
    #[cfg(windows)]
    {
        if imp::device_name().is_some() {
            Availability { available: true, reason: None }
        } else {
            Availability {
                available: false,
                reason: Some(
                    "Pilote d'écran virtuel absent : réinstaller KyberFrog en cochant \
                     « Virtual screen (VDD driver) »."
                        .into(),
                ),
            }
        }
    }
    #[cfg(not(windows))]
    {
        Availability {
            available: false,
            reason: Some("Écran virtuel : Windows uniquement pour l'instant.".into()),
        }
    }
}

/// Make the virtual screen exist, attached, at `size`.
pub fn ensure(size: &VirtualDisplay) -> Result<()> {
    #[cfg(windows)]
    {
        imp::ensure(size)
    }
    #[cfg(not(windows))]
    {
        let _ = size;
        anyhow::bail!("virtual screen: Windows only")
    }
}

/// Detach the virtual screen from the desktop.
pub fn release() {
    #[cfg(windows)]
    imp::release();
}

#[cfg(windows)]
mod imp {
    use anyhow::{bail, Context, Result};
    use log::{info, warn};
    use shared::VirtualDisplay;
    use windows_sys::Win32::Graphics::Gdi::{
        ChangeDisplaySettingsExW, EnumDisplayDevicesW, EnumDisplaySettingsW, CDS_NORESET,
        CDS_UPDATEREGISTRY, DEVMODEW, DISPLAY_DEVICEW, DISP_CHANGE_SUCCESSFUL, DM_PELSHEIGHT,
        DM_PELSWIDTH, DM_POSITION, ENUM_CURRENT_SETTINGS,
    };

    const SETTINGS: &str = r"C:\VirtualDisplayDriver\vdd_settings.xml";
    /// `DeviceString` of the driver's adapter.
    const ADAPTER: &str = "Virtual Display Driver";

    fn wide(s: &[u16]) -> String {
        let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
        String::from_utf16_lossy(&s[..end])
    }

    /// The driver's display device name (`\\.\DISPLAYn`), attached or not.
    /// None when the driver is not installed, or not running.
    pub fn device_name() -> Option<Vec<u16>> {
        let mut index = 0;
        loop {
            // SAFETY: zeroed POD struct with its size set, as the API requires.
            let mut device: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
            device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
            // SAFETY: valid out-pointer; a null device name enumerates adapters.
            if unsafe { EnumDisplayDevicesW(std::ptr::null(), index, &mut device, 0) } == 0 {
                return None;
            }
            if wide(&device.DeviceString).contains(ADAPTER) {
                return Some(device.DeviceName.to_vec());
            }
            index += 1;
        }
    }

    fn devmode() -> DEVMODEW {
        // SAFETY: zeroed POD struct with its size set, as the API requires.
        let mut mode: DEVMODEW = unsafe { std::mem::zeroed() };
        mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
        mode
    }

    /// Does the monitor offer `width`×`height`?
    fn offers(name: &[u16], size: &VirtualDisplay) -> bool {
        let mut mode = devmode();
        let mut index = 0;
        // SAFETY: `name` is a NUL-terminated device name, `mode` a valid out-pointer.
        while unsafe { EnumDisplaySettingsW(name.as_ptr(), index, &mut mode) } != 0 {
            if mode.dmPelsWidth == size.width && mode.dmPelsHeight == size.height {
                return true;
            }
            index += 1;
        }
        false
    }

    /// Current size if attached (a detached monitor has no current mode).
    fn current(name: &[u16]) -> Option<(u32, u32)> {
        let mut mode = devmode();
        // SAFETY: as above.
        let ok = unsafe { EnumDisplaySettingsW(name.as_ptr(), ENUM_CURRENT_SETTINGS, &mut mode) };
        (ok != 0 && mode.dmPelsWidth > 0).then_some((mode.dmPelsWidth, mode.dmPelsHeight))
    }

    /// Set the monitor's size (0×0 detaches it), then apply.
    fn apply(name: &[u16], width: u32, height: u32, x: i32) -> Result<()> {
        let mut mode = devmode();
        mode.dmPelsWidth = width;
        mode.dmPelsHeight = height;
        mode.Anonymous1.Anonymous2.dmPosition.x = x;
        mode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION;
        // SAFETY: valid device name and mode; staged, then applied below.
        let rc = unsafe {
            ChangeDisplaySettingsExW(
                name.as_ptr(),
                &mode,
                std::ptr::null_mut(),
                CDS_UPDATEREGISTRY | CDS_NORESET,
                std::ptr::null(),
            )
        };
        if rc != DISP_CHANGE_SUCCESSFUL {
            bail!("ChangeDisplaySettingsEx({width}x{height}) returned {rc}");
        }
        // SAFETY: null device + null mode = apply the staged changes.
        unsafe {
            ChangeDisplaySettingsExW(
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        };
        Ok(())
    }

    /// Right of every other monitor, so the real desktop does not move.
    fn right_edge() -> i32 {
        crate::monitors::list()
            .iter()
            .map(|m| m.x + m.width)
            .max()
            .unwrap_or(0)
    }

    pub fn ensure(size: &VirtualDisplay) -> Result<()> {
        let name = device_name().context("virtual display driver not installed or not running")?;
        if !offers(&name, size) {
            bail!(
                "virtual display driver does not offer {}x{} (see {SETTINGS})",
                size.width,
                size.height
            );
        }
        match current(&name) {
            Some(now) if now == (size.width, size.height) => return Ok(()),
            // Detach first, so the right edge below is the real monitors'.
            Some(_) => apply(&name, 0, 0, 0)?,
            None => {}
        }
        let x = right_edge();
        apply(&name, size.width, size.height, x)?;
        info!("Virtual display {} attached at {}x{} (x={x})", wide(&name), size.width, size.height);
        Ok(())
    }

    pub fn release() {
        let Some(name) = device_name() else {
            return;
        };
        if current(&name).is_none() {
            return;
        }
        match apply(&name, 0, 0, 0) {
            Ok(()) => info!("Virtual display {} detached", wide(&name)),
            Err(err) => warn!("Virtual display: detach failed: {err:#}"),
        }
    }
}
