// SPDX-License-Identifier: AGPL-3.0-or-later

//! Primary GPU detection for the encoder setting (#28-1).
//!
//! kyavserver (txproto) creates its D3D11 device on the default adapter — DXGI
//! adapter 0 — for Spout and screen capture alike, and derives the hardware
//! encoder from it. So the vendor of adapter 0 is what decides whether AMF or
//! NVENC can work; see `shared::encoder`.
//!
//! Detected once at startup: adapters do not change while KyberFrog runs, and a
//! GPU swap needs a reboot anyway.

use shared::GpuAdapter;

/// The primary GPU, or `None` when there is none (software adapter only), the
/// query fails, or off Windows (the Linux encoders are not auto-selected yet).
pub fn primary_adapter() -> Option<GpuAdapter> {
    imp::primary_adapter()
}

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;

    use log::warn;
    use shared::GpuAdapter;
    use windows_sys::core::GUID;

    // IID_IDXGIFactory1 {770aae78-f26f-4dba-a829-253c83d1b387}
    const IID_IDXGI_FACTORY1: GUID = GUID {
        data1: 0x770a_ae78,
        data2: 0xf26f,
        data3: 0x4dba,
        data4: [0xa8, 0x29, 0x25, 0x3c, 0x83, 0xd1, 0xb3, 0x87],
    };
    const DXGI_ADAPTER_FLAG_SOFTWARE: u32 = 2;

    #[link(name = "dxgi")]
    extern "system" {
        fn CreateDXGIFactory1(riid: *const GUID, factory: *mut *mut c_void) -> i32;
    }

    #[repr(C)]
    struct AdapterDesc1 {
        description: [u16; 128],
        vendor_id: u32,
        device_id: u32,
        sub_sys_id: u32,
        revision: u32,
        dedicated_video_memory: usize,
        dedicated_system_memory: usize,
        shared_system_memory: usize,
        adapter_luid: [u32; 2],
        flags: u32,
    }

    // COM through raw vtables: windows-sys has no interface wrappers. Slots
    // count IUnknown (3) and IDXGIObject (4) first.
    type Release = unsafe extern "system" fn(*mut c_void) -> u32;
    type EnumAdapters1 = unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32;
    type GetDesc1 = unsafe extern "system" fn(*mut c_void, *mut AdapterDesc1) -> i32;
    const SLOT_RELEASE: usize = 2;
    const SLOT_FACTORY_ENUM_ADAPTERS1: usize = 12;
    const SLOT_ADAPTER_GET_DESC1: usize = 10;

    unsafe fn slot<T: Copy>(object: *mut c_void, index: usize) -> T {
        let vtable = *(object as *const *const usize);
        std::mem::transmute_copy(&*vtable.add(index))
    }

    pub fn primary_adapter() -> Option<GpuAdapter> {
        unsafe {
            let mut factory = std::ptr::null_mut();
            if CreateDXGIFactory1(&IID_IDXGI_FACTORY1, &mut factory) < 0 || factory.is_null() {
                warn!("GPU detection: CreateDXGIFactory1 failed");
                return None;
            }
            let mut adapter = std::ptr::null_mut();
            let enum_adapters: EnumAdapters1 = slot(factory, SLOT_FACTORY_ENUM_ADAPTERS1);
            let found = enum_adapters(factory, 0, &mut adapter) >= 0 && !adapter.is_null();
            let mut result = None;
            if found {
                let mut desc: AdapterDesc1 = std::mem::zeroed();
                let get_desc: GetDesc1 = slot(adapter, SLOT_ADAPTER_GET_DESC1);
                if get_desc(adapter, &mut desc) >= 0 && desc.flags & DXGI_ADAPTER_FLAG_SOFTWARE == 0 {
                    let len = desc.description.iter().position(|&c| c == 0).unwrap_or(128);
                    result = Some(GpuAdapter {
                        // The driver string can carry doubled spaces ("AMD  Radeon").
                        name: String::from_utf16_lossy(&desc.description[..len])
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" "),
                        vendor_id: desc.vendor_id,
                    });
                }
                let release: Release = slot(adapter, SLOT_RELEASE);
                release(adapter);
            }
            let release: Release = slot(factory, SLOT_RELEASE);
            release(factory);
            result
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use shared::GpuAdapter;

    pub fn primary_adapter() -> Option<GpuAdapter> {
        None
    }
}
