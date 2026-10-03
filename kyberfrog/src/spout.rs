// SPDX-License-Identifier: AGPL-3.0-or-later

//! Live enumeration of active Spout senders (Windows).
//!
//! Ported from the validated C receiver (`iosys_spout.c`): the Spout SDK keeps
//! the set of active senders in a named shared-memory block `SpoutSenderNames`,
//! laid out as fixed 256-byte, null-terminated name slots. The currently
//! "active" (default) sender name lives in a separate `ActiveSenderName` block.
//!
//! This reads both with `FILE_MAP_READ` only — no SDK, no GPU work — so the app
//! can populate its "add transmitter" picker (tray + web) without touching the
//! senders.

/// Snapshot of the Spout sender registry at one instant.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpoutSenders {
    /// All active sender names, in slot order.
    pub names: Vec<String>,
    /// The sender Spout considers "active" (default), if any.
    pub active: Option<String>,
}

use shared::source::SpoutInfo;

/// Size and pixel format of one sender, read from its own shared-memory info
/// block. `None` when the sender does not exist (or left only its name behind).
pub fn sender_info(name: &str) -> Option<SpoutInfo> {
    #[cfg(windows)]
    {
        imp::info(name).map(|raw| SpoutInfo::new(raw.width, raw.height, raw.format))
    }
    #[cfg(not(windows))]
    {
        let _ = name;
        None
    }
}

/// Read the current set of Spout senders. Returns an empty snapshot when no
/// Spout application has ever run (the shared memory simply does not exist).
///
/// Names that are not real senders are left out: a name without its info
/// block (its app died without cleaning up), and a second name for a
/// texture already listed (TouchDesigner before 2025.33230 registered an
/// extra `<name>_1` for the same texture).
pub fn list_senders() -> SpoutSenders {
    #[cfg(windows)]
    {
        imp::list()
    }
    #[cfg(not(windows))]
    {
        SpoutSenders::default()
    }
}

#[cfg(windows)]
mod imp {
    use super::SpoutSenders;

    use windows_sys::Win32::Foundation::{CloseHandle, FALSE};
    use windows_sys::Win32::System::Memory::{
        MapViewOfFile, OpenFileMappingA, UnmapViewOfFile, VirtualQuery, FILE_MAP_READ,
        MEMORY_BASIC_INFORMATION,
    };

    const SENDER_NAMES_MAP: &[u8] = b"SpoutSenderNames\0";
    const ACTIVE_SENDER_MAP: &[u8] = b"ActiveSenderName\0";
    const NAME_LEN: usize = 256;
    /// Same cap as the C receiver — guards against a bogus RegionSize.
    const MAX_SENDERS: usize = 64;

    /// The first fields of Spout's `SharedTextureInfo`, the per-sender block
    /// named after the sender (`spoutSenderNames.h`).
    #[derive(Clone, Copy)]
    pub(super) struct RawInfo {
        pub share_handle: u32,
        pub width: u32,
        pub height: u32,
        pub format: u32,
    }

    pub(super) fn info(name: &str) -> Option<RawInfo> {
        let mut cname = name.as_bytes().to_vec();
        if cname.contains(&0) || cname.len() >= NAME_LEN {
            return None;
        }
        cname.push(0);
        // Safety: mapping opened read-only, 16 bytes read inside the view,
        // released on every path.
        unsafe {
            let map = OpenFileMappingA(FILE_MAP_READ, FALSE, cname.as_ptr());
            if map.is_null() {
                return None;
            }
            let view = MapViewOfFile(map, FILE_MAP_READ, 0, 0, 16);
            let base = view.Value as *const u32;
            if base.is_null() {
                CloseHandle(map);
                return None;
            }
            let raw = RawInfo {
                share_handle: base.read_unaligned(),
                width: base.add(1).read_unaligned(),
                height: base.add(2).read_unaligned(),
                format: base.add(3).read_unaligned(),
            };
            UnmapViewOfFile(view);
            CloseHandle(map);
            Some(raw)
        }
    }

    pub(super) fn list() -> SpoutSenders {
        let mut handles = Vec::new();
        let names = enumerate_names()
            .into_iter()
            .filter(|name| match info(name) {
                Some(raw) if raw.share_handle != 0 && handles.contains(&raw.share_handle) => false,
                Some(raw) => {
                    handles.push(raw.share_handle);
                    true
                }
                None => false,
            })
            .collect();
        let active = active_sender().filter(|a| !a.is_empty());
        SpoutSenders { names, active }
    }

    fn enumerate_names() -> Vec<String> {
        let mut out = Vec::new();

        // Safety: every pointer is validated before use; the mapping is opened
        // read-only and released on every path.
        unsafe {
            let map = OpenFileMappingA(FILE_MAP_READ, FALSE, SENDER_NAMES_MAP.as_ptr());
            if map.is_null() {
                return out; // No Spout app has ever registered a sender.
            }

            let view = MapViewOfFile(map, FILE_MAP_READ, 0, 0, 0);
            let base = view.Value as *const u8;
            if base.is_null() {
                CloseHandle(map);
                return out;
            }

            // The block size tells us how many name slots exist.
            let mut mbi: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
            let queried = VirtualQuery(
                view.Value as *const _,
                &mut mbi,
                std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
            );
            let nslots = if queried >= std::mem::size_of::<MEMORY_BASIC_INFORMATION>() {
                (mbi.RegionSize / NAME_LEN).min(MAX_SENDERS)
            } else {
                0
            };

            for i in 0..nslots {
                let slot = base.add(i * NAME_LEN);
                if *slot == 0 {
                    continue; // Empty slot.
                }
                out.push(read_cstr(slot, NAME_LEN));
            }

            UnmapViewOfFile(view);
            CloseHandle(map);
        }

        out
    }

    fn active_sender() -> Option<String> {
        // Safety: mapping opened read-only, released on every path.
        unsafe {
            let map = OpenFileMappingA(FILE_MAP_READ, FALSE, ACTIVE_SENDER_MAP.as_ptr());
            if map.is_null() {
                return None;
            }

            let view = MapViewOfFile(map, FILE_MAP_READ, 0, 0, NAME_LEN);
            let base = view.Value as *const u8;
            if base.is_null() {
                CloseHandle(map);
                return None;
            }

            let name = read_cstr(base, NAME_LEN);
            UnmapViewOfFile(view);
            CloseHandle(map);
            Some(name)
        }
    }

    /// Read a null-terminated string of at most `max` bytes from `ptr`.
    ///
    /// Safety: `ptr` must point to at least `max` readable bytes.
    unsafe fn read_cstr(ptr: *const u8, max: usize) -> String {
        let mut len = 0;
        while len < max && *ptr.add(len) != 0 {
            len += 1;
        }
        let bytes = std::slice::from_raw_parts(ptr, len);
        String::from_utf8_lossy(bytes).into_owned()
    }
}
