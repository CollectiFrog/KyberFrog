//! NDI → NDI configuration (NN, plan-bench-latency.md § Étape 5), on the
//! official NDI SDK loaded at run time — the other kybench commands do not
//! need NDI installed.
//!
//! - `kybench ndi-gen`: plays the role of Arena / TouchDesigner with a native
//!   NDI output. The frame (same background, ID bands and HUD as `gen`) is
//!   rendered on the GPU before its deadline; `t_pub` is the deadline; then
//!   the GPU → CPU readback and `NDIlib_send_send_video_v2`, both part of what
//!   such an application pays for NDI, hence part of the NN latency.
//! - `kybench ndi-probe`: finds the source, receives with the SDK, decodes the
//!   ID (`t_out` when the frame is handed over), then uploads the frame into a
//!   GPU texture as any real receiver does (`t_up`).
//!
//! Structures and signatures follow the SDK 6 headers (`Processing.NDI.*.h`).

use std::ffi::{c_char, c_void, CStr, CString};

use windows::core::{PCSTR, PCWSTR};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

use crate::args::Args;
use crate::clock::{now_us, Pacer};
use crate::gen::{self, Background};
use crate::idcode::{self, BAND_H, BAND_W};
use crate::spout::{Device, Res};

#[repr(C)]
struct SendCreate {
    p_ndi_name: *const c_char,
    p_groups: *const c_char,
    clock_video: bool,
    clock_audio: bool,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Source {
    p_ndi_name: *const c_char,
    p_url_address: *const c_char,
}

#[repr(C)]
struct VideoFrame {
    xres: i32,
    yres: i32,
    fourcc: u32,
    frame_rate_n: i32,
    frame_rate_d: i32,
    picture_aspect_ratio: f32,
    frame_format_type: i32,
    timecode: i64,
    p_data: *mut u8,
    line_stride_in_bytes: i32,
    p_metadata: *const c_char,
    timestamp: i64,
}

impl Default for VideoFrame {
    fn default() -> Self {
        Self { xres: 0, yres: 0, fourcc: 0, frame_rate_n: 0, frame_rate_d: 0, picture_aspect_ratio: 0.0,
               frame_format_type: 0, timecode: 0, p_data: std::ptr::null_mut(), line_stride_in_bytes: 0,
               p_metadata: std::ptr::null(), timestamp: 0 }
    }
}

#[repr(C)]
struct FindCreate {
    show_local_sources: bool,
    p_groups: *const c_char,
    p_extra_ips: *const c_char,
}

#[repr(C)]
struct RecvCreate {
    source_to_connect_to: Source,
    color_format: i32,
    bandwidth: i32,
    allow_video_fields: bool,
    p_ndi_recv_name: *const c_char,
}

#[repr(C)]
#[derive(Default)]
struct Performance {
    video_frames: i64,
    audio_frames: i64,
    metadata_frames: i64,
}

type Instance = *mut c_void;

const fn fourcc(s: &[u8; 4]) -> u32 {
    s[0] as u32 | (s[1] as u32) << 8 | (s[2] as u32) << 16 | (s[3] as u32) << 24
}
const FOURCC_UYVY: u32 = fourcc(b"UYVY");
const FOURCC_BGRA: u32 = fourcc(b"BGRA");
const FOURCC_BGRX: u32 = fourcc(b"BGRX");
const FRAME_TYPE_VIDEO: i32 = 1;
const FRAME_TYPE_ERROR: i32 = 4;
const FORMAT_PROGRESSIVE: i32 = 1;
const TIMECODE_SYNTHESIZE: i64 = i64::MAX;
const RECV_COLOR_BGRX_BGRA: i32 = 0;
const RECV_COLOR_UYVY_BGRA: i32 = 1;
const RECV_BANDWIDTH_HIGHEST: i32 = 100;

struct Lib {
    initialize: unsafe extern "C" fn() -> bool,
    version: unsafe extern "C" fn() -> *const c_char,
    send_create: unsafe extern "C" fn(*const SendCreate) -> Instance,
    send_destroy: unsafe extern "C" fn(Instance),
    send_video_v2: unsafe extern "C" fn(Instance, *const VideoFrame),
    send_video_async_v2: unsafe extern "C" fn(Instance, *const VideoFrame),
    send_get_no_connections: unsafe extern "C" fn(Instance, u32) -> i32,
    find_create_v2: unsafe extern "C" fn(*const FindCreate) -> Instance,
    find_destroy: unsafe extern "C" fn(Instance),
    find_wait_for_sources: unsafe extern "C" fn(Instance, u32) -> bool,
    find_get_current_sources: unsafe extern "C" fn(Instance, *mut u32) -> *const Source,
    recv_create_v3: unsafe extern "C" fn(*const RecvCreate) -> Instance,
    recv_destroy: unsafe extern "C" fn(Instance),
    recv_capture_v2: unsafe extern "C" fn(Instance, *mut VideoFrame, *mut c_void, *mut c_void, u32) -> i32,
    recv_free_video_v2: unsafe extern "C" fn(Instance, *const VideoFrame),
    recv_get_performance: unsafe extern "C" fn(Instance, *mut Performance, *mut Performance),
}

macro_rules! sym {
    ($module:expr, $name:literal) => {
        unsafe {
            let f = GetProcAddress($module, PCSTR(concat!($name, "\0").as_ptr()))
                .ok_or(concat!("NDI runtime lacks ", $name))?;
            std::mem::transmute(f)
        }
    };
}

impl Lib {
    /// Load `Processing.NDI.Lib.x64.dll` from `--ndi-dll`, else from the
    /// runtime directory the NDI installers publish (`NDI_RUNTIME_DIR_V6`).
    fn load(a: &Args) -> Res<Self> {
        let path = match a.opt("ndi-dll") {
            Some(p) => p.to_string(),
            None => format!("{}\\Processing.NDI.Lib.x64.dll",
                            std::env::var("NDI_RUNTIME_DIR_V6").map_err(|_| "NDI_RUNTIME_DIR_V6 not set; pass --ndi-dll")?),
        };
        let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        let m = unsafe { LoadLibraryW(PCWSTR(wide.as_ptr())) }.map_err(|e| format!("cannot load {path}: {e}"))?;
        let lib = Self {
            initialize: sym!(m, "NDIlib_initialize"),
            version: sym!(m, "NDIlib_version"),
            send_create: sym!(m, "NDIlib_send_create"),
            send_destroy: sym!(m, "NDIlib_send_destroy"),
            send_video_v2: sym!(m, "NDIlib_send_send_video_v2"),
            send_video_async_v2: sym!(m, "NDIlib_send_send_video_async_v2"),
            send_get_no_connections: sym!(m, "NDIlib_send_get_no_connections"),
            find_create_v2: sym!(m, "NDIlib_find_create_v2"),
            find_destroy: sym!(m, "NDIlib_find_destroy"),
            find_wait_for_sources: sym!(m, "NDIlib_find_wait_for_sources"),
            find_get_current_sources: sym!(m, "NDIlib_find_get_current_sources"),
            recv_create_v3: sym!(m, "NDIlib_recv_create_v3"),
            recv_destroy: sym!(m, "NDIlib_recv_destroy"),
            recv_capture_v2: sym!(m, "NDIlib_recv_capture_v2"),
            recv_free_video_v2: sym!(m, "NDIlib_recv_free_video_v2"),
            recv_get_performance: sym!(m, "NDIlib_recv_get_performance"),
        };
        if !unsafe { (lib.initialize)() } {
            return Err("NDIlib_initialize failed (unsupported CPU?)".into());
        }
        eprintln!("ndi: runtime {} ({path})", lib.version_string());
        Ok(lib)
    }

    fn version_string(&self) -> String {
        unsafe { CStr::from_ptr((self.version)()).to_string_lossy().into_owned() }
    }
}

fn write_csv(path: &str, header: &str, rows: &[String]) -> Res<()> {
    let mut out = String::from(header);
    for r in rows {
        out.push_str(r);
        out.push('\n');
    }
    std::fs::write(path, out)?;
    Ok(())
}

// --- sender ------------------------------------------------------------------

pub fn run_gen(a: &Args) -> Res<()> {
    let name = a.str("name", "kybench-ndi");
    let (w, h) = (a.num("width", 1920usize)?, a.num("height", 1080usize)?);
    let fps = a.num("fps", 60u64)?;
    let duration_s = a.num("duration", 60.0f64)?;
    let seed = a.num("seed", 1u64)?;
    let csv = a.str("csv", "ndi-gen.csv");
    let fourcc = match a.str("fourcc", "BGRX").as_str() {
        "BGRX" => FOURCC_BGRX,
        "BGRA" => FOURCC_BGRA,
        other => return Err(format!("--fourcc {other}: BGRX or BGRA").into()),
    };

    // `--clock true`: the SDK paces `send` to the frame rate (blocks);
    // default false, the generator's timer paces. `--async true`: the frame is
    // copied out of the mapped staging texture into one of two alternating
    // buffers (the SDK keeps using it until the next async call) and sent
    // with `NDIlib_send_send_video_async_v2`.
    let clock = a.str("clock", "false") == "true";
    let scroll = match a.opt("scroll") {
        Some(_) => Some((a.num("scroll", 8usize)?, a.num("scroll-offset", 0usize)?)),
        None => None,
    };
    let asynchronous = a.str("async", "false") == "true";

    let lib = Lib::load(a)?;
    let dev = Device::new()?;
    let private = dev.private_texture(w as u32, h as u32)?;
    let staging = dev.staging_texture(w as u32, h as u32)?;
    let background = Background::new(w, h, seed);
    let mut frame = vec![0u8; w * h * 4];
    let mut owned = [vec![0u8; w * h * 4], vec![0u8; w * h * 4]];
    let origins = idcode::band_origins(w, h);
    let pacer = Pacer::new()?;

    let cname = CString::new(name.clone())?;
    let settings = SendCreate { p_ndi_name: cname.as_ptr(), p_groups: std::ptr::null(),
                                clock_video: clock, clock_audio: false };
    let send = unsafe { (lib.send_create)(&settings) };
    if send.is_null() {
        return Err("NDIlib_send_create failed".into());
    }

    let start = now_us() + 500_000;
    let end = start + (duration_s * 1e6) as i64;
    let mut rows = Vec::with_capacity((duration_s * fps as f64) as usize + 1);
    eprintln!("ndi-gen: '{name}' {w}x{h}@{fps} {}, {duration_s} s, seed {seed}, clock {clock}, async {asynchronous}",
              String::from_utf8_lossy(&fourcc.to_le_bytes()));

    let result = (|| -> Res<()> {
        for n in 0u64.. {
            let deadline = start + (n * 1_000_000 / fps) as i64;
            if deadline >= end {
                break;
            }
            // The application's frame: rendered on the GPU before the deadline.
            let id = gen::render(&background, &mut frame, w, h, n, fps, &origins);
            if let Some((scroll, offset)) = scroll {
                // Diagnostic only: redraw the background with another motion.
                background.compose_scrolled(n, &mut frame, w, scroll, offset);
                for &(x, y) in &origins {
                    idcode::encode(&mut frame, w * 4, x, y, id);
                }
                crate::hud::draw(&mut frame, w * 4, w, h, n, fps);
            }
            dev.upload(&private, &frame, w * 4)?;
            dev.finish()?;

            pacer.sleep_until(deadline);
            let t_pub = now_us();
            dev.copy(&staging, &private)?;
            let video = |p_data: *mut u8, stride: usize| VideoFrame {
                xres: w as i32, yres: h as i32, fourcc,
                frame_rate_n: fps as i32, frame_rate_d: 1,
                frame_format_type: FORMAT_PROGRESSIVE, timecode: TIMECODE_SYNTHESIZE,
                p_data, line_stride_in_bytes: stride as i32,
                ..Default::default()
            };
            let (t_read, t_sent) = if asynchronous {
                let buf = &mut owned[(n % 2) as usize];
                dev.read(&staging, |bytes, pitch| {
                    for (y, row) in buf.chunks_exact_mut(w * 4).enumerate() {
                        row.copy_from_slice(&bytes[y * pitch..y * pitch + w * 4]);
                    }
                })?;
                let t_read = now_us();
                unsafe { (lib.send_video_async_v2)(send, &video(buf.as_mut_ptr(), w * 4)) };
                (t_read, now_us())
            } else {
                dev.read(&staging, |bytes, pitch| {
                    let t_read = now_us();
                    unsafe { (lib.send_video_v2)(send, &video(bytes.as_ptr() as *mut u8, pitch)) };
                    (t_read, now_us())
                })?
            };
            let connections = if n % fps == 0 { unsafe { (lib.send_get_no_connections)(send, 0) } } else { -1 };
            rows.push(format!("{id},{deadline},{t_pub},1,{t_read},{t_sent},{connections}"));
        }
        Ok(())
    })();
    unsafe {
        if asynchronous {
            // A null frame synchronises: the SDK releases the last buffer.
            (lib.send_video_async_v2)(send, std::ptr::null());
        }
        (lib.send_destroy)(send);
    }
    result?;

    write_csv(&csv, "id,t_deadline_us,t_pub_us,published,t_read_us,t_sent_us,connections\n", &rows)?;
    eprintln!("ndi-gen: {} frames -> {csv}", rows.len());
    Ok(())
}

/// `kybench ndi-list`: the NDI sources visible after `--wait` seconds.
pub fn run_list(a: &Args) -> Res<()> {
    let wait_s = a.num("wait", 3.0f64)?;
    let lib = Lib::load(a)?;
    let settings = FindCreate { show_local_sources: true, p_groups: std::ptr::null(), p_extra_ips: std::ptr::null() };
    let find = unsafe { (lib.find_create_v2)(&settings) };
    if find.is_null() {
        return Err("NDIlib_find_create_v2 failed".into());
    }
    let end = now_us() + (wait_s * 1e6) as i64;
    while now_us() < end {
        unsafe { (lib.find_wait_for_sources)(find, 500) };
    }
    let mut count = 0u32;
    let list = unsafe { (lib.find_get_current_sources)(find, &mut count) };
    for i in 0..count as usize {
        let s = unsafe { *list.add(i) };
        let url = if s.p_url_address.is_null() { "".into() } else { unsafe { CStr::from_ptr(s.p_url_address) }.to_string_lossy() };
        println!("{} | {url}", unsafe { CStr::from_ptr(s.p_ndi_name) }.to_string_lossy());
    }
    unsafe { (lib.find_destroy)(find) };
    Ok(())
}

// --- receiver ----------------------------------------------------------------

/// Copy the ID band at (`x0`, `y0`) out of a received frame as BGRA; UYVY is
/// read through its luma samples (Y of pixel x at byte 2x + 1).
fn extract_band(data: &[u8], stride: usize, fourcc: u32, x0: usize, y0: usize) -> Vec<u8> {
    let mut band = vec![0u8; BAND_W * BAND_H * 4];
    for y in 0..BAND_H {
        let row = &data[(y0 + y) * stride..];
        for x in 0..BAND_W {
            let o = (y * BAND_W + x) * 4;
            if fourcc == FOURCC_UYVY {
                let v = row[2 * (x0 + x) + 1];
                band[o..o + 4].copy_from_slice(&[v, v, v, 255]);
            } else {
                band[o..o + 4].copy_from_slice(&row[(x0 + x) * 4..(x0 + x) * 4 + 4]);
            }
        }
    }
    band
}

fn fmt(r: &Result<u32, idcode::DecodeError>) -> String {
    match r {
        Ok(id) => id.to_string(),
        Err(e) => format!("{e:?}"),
    }
}

pub fn run_probe(a: &Args) -> Res<()> {
    let source = a.str("source", "kybench-ndi");
    let duration_s = a.num("duration", 60.0f64)?;
    let wait_s = a.num("wait", 30.0f64)?;
    let csv = a.str("csv", "ndi-probe.csv");
    let upload = a.str("upload", "true") == "true";
    let color = match a.str("color", "uyvy").as_str() {
        "uyvy" => RECV_COLOR_UYVY_BGRA,
        "bgra" => RECV_COLOR_BGRX_BGRA,
        other => return Err(format!("--color {other}: uyvy or bgra").into()),
    };

    let lib = Lib::load(a)?;
    let dev = Device::new()?;
    let _priority = Pacer::new()?;

    // Discovery: the advertised name is "MACHINE (source)".
    let find_settings = FindCreate { show_local_sources: true, p_groups: std::ptr::null(),
                                     p_extra_ips: std::ptr::null() };
    let find = unsafe { (lib.find_create_v2)(&find_settings) };
    if find.is_null() {
        return Err("NDIlib_find_create_v2 failed".into());
    }
    let suffix = format!("({source})");
    let give_up = now_us() + (wait_s * 1e6) as i64;
    let found = loop {
        unsafe { (lib.find_wait_for_sources)(find, 500) };
        let mut count = 0u32;
        let list = unsafe { (lib.find_get_current_sources)(find, &mut count) };
        let hit = (0..count as usize).map(|i| unsafe { *list.add(i) })
            .find(|s| unsafe { CStr::from_ptr(s.p_ndi_name) }.to_string_lossy().ends_with(&suffix));
        if let Some(s) = hit {
            break Some(s);
        }
        if now_us() > give_up {
            break None;
        }
    };
    let Some(found) = found else {
        unsafe { (lib.find_destroy)(find) };
        return Err(format!("NDI source '{suffix}' not found in {wait_s} s").into());
    };
    let full_name = unsafe { CStr::from_ptr(found.p_ndi_name) }.to_string_lossy().into_owned();

    let recv_name = CString::new("kybench-ndi-probe")?;
    let settings = RecvCreate { source_to_connect_to: found, color_format: color,
                                bandwidth: RECV_BANDWIDTH_HIGHEST, allow_video_fields: false,
                                p_ndi_recv_name: recv_name.as_ptr() };
    let recv = unsafe { (lib.recv_create_v3)(&settings) };
    unsafe { (lib.find_destroy)(find) };
    if recv.is_null() {
        return Err("NDIlib_recv_create_v3 failed".into());
    }
    eprintln!("ndi-probe: '{full_name}', {duration_s} s, color {}, upload {upload}", a.str("color", "uyvy"));

    let mut rows = Vec::new();
    let mut texture = None;
    let mut received = 0u64;
    let mut formats = std::collections::BTreeSet::new();
    let end = now_us() + (duration_s * 1e6) as i64;
    let result = (|| -> Res<()> {
        while now_us() < end {
            let mut video = VideoFrame::default();
            let kind = unsafe {
                (lib.recv_capture_v2)(recv, &mut video, std::ptr::null_mut(), std::ptr::null_mut(), 100)
            };
            if kind == FRAME_TYPE_ERROR {
                return Err("NDI receiver lost the connection".into());
            }
            if kind != FRAME_TYPE_VIDEO {
                continue;
            }
            let t_out = now_us();
            received += 1;
            let (w, h, stride) = (video.xres as usize, video.yres as usize, video.line_stride_in_bytes as usize);
            formats.insert((String::from_utf8_lossy(&video.fourcc.to_le_bytes()).into_owned(), w, h));
            let data = unsafe { std::slice::from_raw_parts(video.p_data, stride * h) };
            let ids: Vec<_> = idcode::band_origins(w, h).iter()
                .map(|&(x, y)| idcode::decode(&extract_band(data, stride, video.fourcc, x, y), BAND_W * 4))
                .collect();
            let t_read = now_us();
            let t_up = if upload {
                // The bytes as delivered (UYVY: 4 bytes per 2 pixels, a
                // half-width BGRA texture), as a renderer would upload them
                // before converting on the GPU.
                let tw = if video.fourcc == FOURCC_UYVY { w / 2 } else { w } as u32;
                if texture.as_ref().map_or(true, |(tex_w, tex_h, _)| (*tex_w, *tex_h) != (tw, h as u32)) {
                    texture = Some((tw, h as u32, dev.private_texture(tw, h as u32)?));
                }
                dev.upload(&texture.as_ref().unwrap().2, data, stride)?;
                dev.finish()?;
                now_us()
            } else {
                t_read
            };
            let ok = matches!((ids[0], ids[1]), (Ok(x), Ok(y)) if x == y);
            rows.push(format!("{received},1,{t_out},{t_read},{},{},{},{t_up},{}", fmt(&ids[0]), fmt(&ids[1]),
                              ok as u8, video.timestamp));
            unsafe { (lib.recv_free_video_v2)(recv, &video) };
        }
        Ok(())
    })();
    let (mut total, mut dropped) = (Performance::default(), Performance::default());
    unsafe {
        (lib.recv_get_performance)(recv, &mut total, &mut dropped);
        (lib.recv_destroy)(recv);
    }
    result?;

    write_csv(&csv, "frame_count,step,t_out_us,t_read_us,id_a,id_b,decoded,t_up_us,ndi_timestamp\n", &rows)?;
    let summary = format!(
        "{{\"source\": \"{}\", \"runtime\": \"{}\", \"color\": \"{}\", \"upload\": {upload}, \
         \"formats\": \"{}\", \"sdk_video_frames\": {}, \"sdk_video_dropped\": {}}}\n",
        full_name.replace('\\', "\\\\").replace('"', "'"), lib.version_string(), a.str("color", "uyvy"),
        format!("{formats:?}").replace('"', "'"), total.video_frames, dropped.video_frames);
    std::fs::write(format!("{csv}.json"), &summary)?;
    let decoded = rows.iter().filter(|r| r.split(',').nth(6) == Some("1")).count();
    eprintln!("ndi-probe: {} frames, {decoded} decoded, SDK dropped {} -> {csv}", rows.len(), dropped.video_frames);
    Ok(())
}
