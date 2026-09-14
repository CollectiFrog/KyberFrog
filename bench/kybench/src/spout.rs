//! Spout2 registry, sender and receiver — a standalone reimplementation of the
//! protocol used by kyspout (sender) and txproto `iosys_spout.c` (receiver), so
//! the bench does not depend on the fork chain:
//!
//! - `SpoutSenderNames`: flat array of 256-byte name slots;
//! - `ActiveSenderName`: the default sender;
//! - `<name>`: `SharedTextureInfo` (280 bytes: share handle, size, DXGI format);
//! - `<name>SpoutAccessMutex`: guards texture access;
//! - `<name>_Count_Semaphore`: frame counter, bumped once per published frame.

use std::error::Error;
use std::ffi::c_void;

use windows::core::{ComInterface, PCSTR};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HMODULE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Asynchronous, ID3D11Device, ID3D11DeviceContext, ID3D11Resource,
    ID3D11Texture2D, D3D11_BIND_SHADER_RESOURCE, D3D11_BOX, D3D11_QUERY_DESC, D3D11_QUERY_EVENT, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_RESOURCE_MISC_SHARED, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::IDXGIResource;
use windows::Win32::System::Memory::{
    CreateFileMappingA, MapViewOfFile, OpenFileMappingA, UnmapViewOfFile, FILE_MAP_ALL_ACCESS,
    FILE_MAP_READ, MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    CreateMutexA, CreateSemaphoreA, ReleaseMutex, ReleaseSemaphore, WaitForSingleObject,
};

// Not exposed by the `windows` 0.52 bindings; plain kernel32 exports.
#[link(name = "kernel32")]
extern "system" {
    fn OpenMutexA(access: u32, inherit: i32, name: *const u8) -> isize;
    fn OpenSemaphoreA(access: u32, inherit: i32, name: *const u8) -> isize;
}
// Non-destructive semaphore count (SemaphoreBasicInformation = 0). The Spout
// SDK pattern — WaitForSingleObject(0) then ReleaseSemaphore — is not atomic:
// two pollers on one sender each see the other's transient decrement as a
// counter change (observed: kyavserver capturing ~900 phantom frames/s while
// a probe polled the same sender). Querying never touches the count.
#[link(name = "ntdll")]
extern "system" {
    fn NtQuerySemaphore(handle: isize, class: u32, info: *mut SemaphoreBasicInformation, len: u32,
                        written: *mut u32) -> i32;
}
#[repr(C)]
#[derive(Default)]
struct SemaphoreBasicInformation {
    current: u32,
    maximum: u32,
}
const SYNCHRONIZE: u32 = 0x0010_0000;
const SEMAPHORE_QUERY_STATE: u32 = 0x0001;
const SEMAPHORE_MODIFY_STATE: u32 = 0x0002;

pub type Res<T> = Result<T, Box<dyn Error>>;

const NAME_LEN: usize = 256;
const MAX_SENDERS: usize = 256;
const INFO_LEN: usize = 280;
/// Spout SDK timeout on the per-sender access mutex.
const MUTEX_TIMEOUT_MS: u32 = 67;
const DXGI_BGRA: u32 = 87;

fn cstr(s: &str) -> Vec<u8> {
    let mut v = s.as_bytes().to_vec();
    v.push(0);
    v
}

pub struct Device {
    pub device: ID3D11Device,
    pub context: ID3D11DeviceContext,
}

impl Device {
    pub fn new() -> Res<Self> {
        let mut device = None;
        let mut context = None;
        unsafe {
            D3D11CreateDevice(None, D3D_DRIVER_TYPE_HARDWARE, HMODULE::default(),
                              D3D11_CREATE_DEVICE_BGRA_SUPPORT, Some(&[D3D_FEATURE_LEVEL_11_0]),
                              D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut context))?;
        }
        Ok(Self { device: device.ok_or("no device")?, context: context.ok_or("no context")? })
    }

    fn texture(&self, w: u32, h: u32, staging: bool, shared: bool) -> Res<ID3D11Texture2D> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: w,
            Height: h,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: if staging { D3D11_USAGE_STAGING } else { D3D11_USAGE_DEFAULT },
            BindFlags: if staging { 0 } else { D3D11_BIND_SHADER_RESOURCE.0 as u32 },
            CPUAccessFlags: if staging { D3D11_CPU_ACCESS_READ.0 as u32 } else { 0 },
            MiscFlags: if shared { D3D11_RESOURCE_MISC_SHARED.0 as u32 } else { 0 },
        };
        let mut tex = None;
        unsafe { self.device.CreateTexture2D(&desc, None, Some(&mut tex))? };
        Ok(tex.ok_or("no texture")?)
    }

    /// GPU-only BGRA texture.
    pub fn private_texture(&self, w: u32, h: u32) -> Res<ID3D11Texture2D> {
        self.texture(w, h, false, false)
    }

    /// CPU-readable BGRA texture.
    pub fn staging_texture(&self, w: u32, h: u32) -> Res<ID3D11Texture2D> {
        self.texture(w, h, true, false)
    }

    pub fn upload(&self, tex: &ID3D11Texture2D, bgra: &[u8], stride: usize) -> Res<()> {
        let r: ID3D11Resource = tex.cast()?;
        unsafe {
            self.context.UpdateSubresource(&r, 0, None, bgra.as_ptr() as *const c_void,
                                           stride as u32, 0)
        };
        Ok(())
    }

    pub fn copy(&self, dst: &ID3D11Texture2D, src: &ID3D11Texture2D) -> Res<()> {
        let (d, s): (ID3D11Resource, ID3D11Resource) = (dst.cast()?, src.cast()?);
        unsafe { self.context.CopyResource(&d, &s) };
        Ok(())
    }

    /// Copy the `w`×`h` region at (`x`, `y`) of `src` to the origin of `dst`.
    pub fn copy_region(&self, dst: &ID3D11Texture2D, src: &ID3D11Texture2D, x: u32, y: u32,
                       w: u32, h: u32) -> Res<()> {
        let (d, s): (ID3D11Resource, ID3D11Resource) = (dst.cast()?, src.cast()?);
        let bx = D3D11_BOX { left: x, top: y, front: 0, right: x + w, bottom: y + h, back: 1 };
        unsafe { self.context.CopySubresourceRegion(&d, 0, 0, 0, 0, &s, 0, Some(&bx)) };
        Ok(())
    }

    /// Map a staging texture for reading (blocks until the GPU has written it)
    /// and hand its bytes and row pitch to `f`.
    pub fn read<T>(&self, tex: &ID3D11Texture2D, f: impl FnOnce(&[u8], usize) -> T) -> Res<T> {
        let r: ID3D11Resource = tex.cast()?;
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { tex.GetDesc(&mut desc) };
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe { self.context.Map(&r, 0, D3D11_MAP_READ, 0, Some(&mut mapped))? };
        let pitch = mapped.RowPitch as usize;
        let len = pitch * (desc.Height as usize - 1) + desc.Width as usize * 4;
        let bytes = unsafe { std::slice::from_raw_parts(mapped.pData as *const u8, len) };
        let out = f(bytes, pitch);
        unsafe { self.context.Unmap(&r, 0) };
        Ok(out)
    }

    pub fn flush(&self) {
        unsafe { self.context.Flush() };
    }

    /// Flush and block until the GPU has executed every queued command.
    pub fn finish(&self) -> Res<()> {
        let desc = D3D11_QUERY_DESC { Query: D3D11_QUERY_EVENT, MiscFlags: 0 };
        let mut query = None;
        unsafe { self.device.CreateQuery(&desc, Some(&mut query))? };
        let query: ID3D11Asynchronous = query.ok_or("no query")?.cast()?;
        unsafe {
            self.context.End(&query);
            self.context.Flush();
            let mut done = 0i32;
            loop {
                self.context.GetData(&query, Some(&mut done as *mut i32 as *mut c_void), 4, 0)?;
                if done != 0 {
                    return Ok(());
                }
                std::thread::yield_now();
            }
        }
    }
}

struct Mapping {
    handle: HANDLE,
    view: *mut u8,
}

impl Mapping {
    fn create(name: &str, size: usize) -> Res<Self> {
        let n = cstr(name);
        unsafe {
            let handle = CreateFileMappingA(INVALID_HANDLE_VALUE, None, PAGE_READWRITE, 0,
                                            size as u32, PCSTR(n.as_ptr()))?;
            Self::map(handle, FILE_MAP_ALL_ACCESS.0, size)
        }
    }

    fn open(name: &str, access: u32, size: usize) -> Res<Self> {
        let n = cstr(name);
        unsafe {
            let handle = OpenFileMappingA(access, false, PCSTR(n.as_ptr()))?;
            Self::map(handle, access, size)
        }
    }

    unsafe fn map(handle: HANDLE, access: u32, size: usize) -> Res<Self> {
        let addr = MapViewOfFile(handle, windows::Win32::System::Memory::FILE_MAP(access), 0, 0, size);
        if addr.Value.is_null() {
            let _ = CloseHandle(handle);
            return Err(windows::core::Error::from_win32().into());
        }
        Ok(Self { handle, view: addr.Value as *mut u8 })
    }
}

impl Drop for Mapping {
    fn drop(&mut self) {
        unsafe {
            let _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS { Value: self.view as *mut c_void });
            let _ = CloseHandle(self.handle);
        }
    }
}

unsafe fn read_name(src: *const u8) -> String {
    let bytes = std::slice::from_raw_parts(src, NAME_LEN);
    let len = bytes.iter().position(|&b| b == 0).unwrap_or(NAME_LEN);
    String::from_utf8_lossy(&bytes[..len]).into_owned()
}

unsafe fn write_name(dst: *mut u8, name: &str) {
    std::ptr::write_bytes(dst, 0, NAME_LEN);
    std::ptr::copy_nonoverlapping(name.as_ptr(), dst, name.len().min(NAME_LEN - 1));
}

pub struct Sender {
    name: String,
    texture: ID3D11Texture2D,
    names: Mapping,
    _active: Mapping,
    _info: Mapping,
    mutex: HANDLE,
    semaphore: HANDLE,
}

impl Sender {
    pub fn new(dev: &Device, name: &str, w: u32, h: u32) -> Res<Self> {
        if name.is_empty() || name.len() >= NAME_LEN {
            return Err("sender name must be 1..=255 bytes".into());
        }
        let texture = dev.texture(w, h, false, true)?;
        let handle = unsafe { texture.cast::<IDXGIResource>()?.GetSharedHandle()? };

        let names = Mapping::open("SpoutSenderNames", FILE_MAP_ALL_ACCESS.0, MAX_SENDERS * NAME_LEN)
            .or_else(|_| Mapping::create("SpoutSenderNames", MAX_SENDERS * NAME_LEN))?;
        unsafe {
            let slots: Vec<String> = (0..MAX_SENDERS).map(|i| read_name(names.view.add(i * NAME_LEN))).collect();
            if slots.iter().any(|s| s == name) {
                return Err(format!("a Spout sender named '{name}' already exists").into());
            }
            let free = slots.iter().position(|s| s.is_empty()).ok_or("Spout registry full")?;
            write_name(names.view.add(free * NAME_LEN), name);
        }
        let active = Mapping::create("ActiveSenderName", NAME_LEN)?;
        unsafe { write_name(active.view, name) };

        let info = Mapping::create(name, INFO_LEN)?;
        unsafe {
            std::ptr::write_bytes(info.view, 0, INFO_LEN);
            for (i, v) in [handle.0 as usize as u32, w, h, DXGI_BGRA, 0].iter().enumerate() {
                std::ptr::write_unaligned(info.view.add(i * 4) as *mut u32, *v);
            }
        }
        let (m, s) = (cstr(&format!("{name}SpoutAccessMutex")), cstr(&format!("{name}_Count_Semaphore")));
        let (mutex, semaphore) = unsafe {
            (CreateMutexA(None, false, PCSTR(m.as_ptr()))?,
             CreateSemaphoreA(None, 0, i32::MAX, PCSTR(s.as_ptr()))?)
        };
        Ok(Self { name: name.to_string(), texture, names, _active: active, _info: info, mutex, semaphore })
    }

    /// Publish the content of `src` (same device, same size): [`Sender::prepare`],
    /// then — if `due` is given — wait until that QPC instant, then
    /// [`Sender::signal`]. The GPU work is thus paid *before* the deadline and
    /// `t_pub` lands on it within the pacer's precision.
    ///
    /// Returns `t_pub`; None if the access mutex stayed busy.
    pub fn publish_at(&self, dev: &Device, src: &ID3D11Texture2D,
                      due: Option<(i64, &crate::clock::Pacer)>) -> Res<Option<i64>> {
        if !self.prepare(dev, src)? {
            return Ok(None);
        }
        if let Some((due, pacer)) = due {
            pacer.sleep_until(due);
        }
        Ok(Some(self.signal()))
    }

    /// Lock the sender, copy `src` into the shared texture and wait for the GPU
    /// to have executed the copy. The mutex stays held until [`Sender::signal`].
    /// False if the mutex stayed busy (nothing held).
    ///
    /// Waiting for the GPU matters: after a mere `Flush` a receiver on another
    /// device can still read the previous frame (observed).
    pub fn prepare(&self, dev: &Device, src: &ID3D11Texture2D) -> Res<bool> {
        unsafe {
            if WaitForSingleObject(self.mutex, MUTEX_TIMEOUT_MS) != WAIT_OBJECT_0 {
                return Ok(false);
            }
        }
        if let Err(e) = dev.copy(&self.texture, src).and_then(|_| dev.finish()) {
            unsafe { let _ = ReleaseMutex(self.mutex); }
            return Err(e);
        }
        Ok(true)
    }

    /// Take `t_pub`, unlock and bump the frame counter: a receiver cannot see
    /// the frame earlier, so latencies are never negative. Only after
    /// [`Sender::prepare`] returned true.
    pub fn signal(&self) -> i64 {
        let t_pub = crate::clock::now_us();
        unsafe {
            let _ = ReleaseMutex(self.mutex);
            let _ = ReleaseSemaphore(self.semaphore, 1, None);
        }
        t_pub
    }
}

impl Drop for Sender {
    fn drop(&mut self) {
        unsafe {
            for i in 0..MAX_SENDERS {
                let slot = self.names.view.add(i * NAME_LEN);
                if read_name(slot) == self.name {
                    std::ptr::write_bytes(slot, 0, NAME_LEN);
                    break;
                }
            }
            let _ = CloseHandle(self.mutex);
            let _ = CloseHandle(self.semaphore);
        }
    }
}

pub fn sender_names() -> Vec<String> {
    match Mapping::open("SpoutSenderNames", FILE_MAP_READ.0, MAX_SENDERS * NAME_LEN) {
        Ok(map) => (0..MAX_SENDERS)
            .map(|i| unsafe { read_name(map.view.add(i * NAME_LEN)) })
            .take_while(|s| !s.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

pub struct Receiver {
    name: String,
    handle: u32,
    pub width: u32,
    pub height: u32,
    texture: Option<ID3D11Texture2D>,
    mutex: HANDLE,
    semaphore: HANDLE,
    /// Read the counter the Spout SDK way (wait then release) instead of
    /// querying it: emulates a real SDK receiver (or kyavserver) next to the
    /// probe — for measuring what such a neighbour costs, never for measuring.
    sdk_poll: bool,
}

impl Receiver {
    pub fn new(name: &str, sdk_poll: bool) -> Self {
        Self { name: name.to_string(), handle: 0, width: 0, height: 0, texture: None,
               mutex: HANDLE::default(), semaphore: HANDLE::default(), sdk_poll }
    }

    fn info(&self) -> Option<[u32; 4]> {
        let map = Mapping::open(&self.name, FILE_MAP_READ.0, INFO_LEN).ok()?;
        let v = |i: usize| unsafe { std::ptr::read_unaligned(map.view.add(i * 4) as *const u32) };
        Some([v(0), v(1), v(2), v(3)])
    }

    /// (Re)bind the sender's texture if it appeared or changed. True when bound.
    pub fn refresh(&mut self, dev: &Device) -> Res<bool> {
        let Some([handle, w, h, _fmt]) = self.info() else {
            self.texture = None;
            return Ok(false);
        };
        if handle == 0 || w == 0 || h == 0 {
            return Ok(false);
        }
        if self.texture.is_none() || handle != self.handle {
            let mut tex: Option<ID3D11Texture2D> = None;
            unsafe { dev.device.OpenSharedResource(HANDLE(handle as isize), &mut tex)? };
            self.texture = Some(tex.ok_or("OpenSharedResource returned null")?);
            (self.handle, self.width, self.height) = (handle, w, h);
        }
        unsafe {
            if self.mutex.is_invalid() {
                let n = cstr(&format!("{}SpoutAccessMutex", self.name));
                self.mutex = HANDLE(OpenMutexA(SYNCHRONIZE, 0, n.as_ptr()));
            }
            if self.semaphore.is_invalid() {
                let n = cstr(&format!("{}_Count_Semaphore", self.name));
                self.semaphore = HANDLE(OpenSemaphoreA(
                    SYNCHRONIZE | SEMAPHORE_QUERY_STATE | SEMAPHORE_MODIFY_STATE, 0, n.as_ptr()));
            }
        }
        Ok(true)
    }

    pub fn texture(&self) -> Option<&ID3D11Texture2D> {
        self.texture.as_ref()
    }

    /// Current frame counter, read without touching it (`NtQuerySemaphore`);
    /// falls back to the Spout SDK wait/release pattern if the query fails.
    pub fn frame_count(&self) -> Option<i64> {
        if self.semaphore.is_invalid() {
            return None;
        }
        unsafe {
            let mut info = SemaphoreBasicInformation::default();
            let mut written = 0u32;
            if !self.sdk_poll && NtQuerySemaphore(self.semaphore.0, 0, &mut info, 8, &mut written) == 0 {
                return Some(info.current as i64);
            }
            if WaitForSingleObject(self.semaphore, 0) != WAIT_OBJECT_0 {
                return Some(0);
            }
            let mut prev = 0i32;
            let _ = ReleaseSemaphore(self.semaphore, 1, Some(&mut prev));
            Some(prev as i64 + 1)
        }
    }

    /// Baseline for change detection: the maximum of a short burst of reads,
    /// so a concurrent Spout-SDK poller's transient decrement (a few µs) cannot
    /// seed the baseline one below the truth — that would make the next read
    /// look like a new frame.
    pub fn settled_frame_count(&self, pacer: &crate::clock::Pacer) -> Option<i64> {
        let mut best = self.frame_count()?;
        for _ in 0..20 {
            pacer.sleep_until(crate::clock::now_us() + 100);
            best = best.max(self.frame_count()?);
        }
        Some(best)
    }

    /// Run `f` under the sender's access mutex (or without it if the sender has
    /// none). None if the mutex stayed busy.
    pub fn locked<T>(&self, f: impl FnOnce() -> T) -> Option<T> {
        if self.mutex.is_invalid() {
            return Some(f());
        }
        unsafe {
            if WaitForSingleObject(self.mutex, MUTEX_TIMEOUT_MS) != WAIT_OBJECT_0 {
                return None;
            }
            let out = f();
            let _ = ReleaseMutex(self.mutex);
            Some(out)
        }
    }
}

impl Drop for Receiver {
    fn drop(&mut self) {
        unsafe {
            if !self.mutex.is_invalid() {
                let _ = CloseHandle(self.mutex);
            }
            if !self.semaphore.is_invalid() {
                let _ = CloseHandle(self.semaphore);
            }
        }
    }
}
