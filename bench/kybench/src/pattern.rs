//! Colour bars in any DXGI format, and the check that reads them back — the
//! test bed of the pixel-format audit (docs/dev/audit-resolutions.md, C1).
//!
//!   kybench pattern --name kf-fmt-src --format rgba16f --width 1280 --height 720 --duration 60
//!   kybench check   --name bench-out --expect rgba16f --wait 60 [--tolerance 40]
//!
//! `pattern` publishes four vertical bars (red, green, blue, 50 % grey; a mono
//! format gets four grey levels instead) in the format a real sender would use
//! — TouchDesigner's 16-bit float, Unreal's 10-bit — and bumps the frame
//! counter at `--fps`. `check` reads another sender (kyclient's Spout output),
//! averages a block at the centre of each bar and compares it to what the
//! bars should look like in 8-bit. Exit code 0 only when every bar matches.

use windows::Win32::Graphics::Direct3D11::D3D11_TEXTURE2D_DESC;

use crate::args::Args;
use crate::clock::{now_us, Pacer};
use crate::spout::{Device, Receiver, Res, Sender};

/// The colour bars, linear 0..1 RGBA.
const BARS: [[f32; 4]; 4] = [[1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0],
                             [0.5, 0.5, 0.5, 1.0]];
/// The grey level of each bar when the format has a single channel.
const MONO: [f32; 4] = [1.0, 0.0, 0.5, 0.25];

struct Format {
    id: u32,
    name: &'static str,
    bytes: usize,
    mono: bool,
}

/// Formats a Spout sender can publish, by the names the bench accepts.
const FORMATS: &[Format] = &[
    Format { id: 87, name: "bgra8", bytes: 4, mono: false },
    Format { id: 28, name: "rgba8", bytes: 4, mono: false },
    Format { id: 88, name: "bgrx8", bytes: 4, mono: false },
    Format { id: 91, name: "bgra8srgb", bytes: 4, mono: false },
    Format { id: 29, name: "rgba8srgb", bytes: 4, mono: false },
    Format { id: 24, name: "rgb10a2", bytes: 4, mono: false },
    Format { id: 10, name: "rgba16f", bytes: 8, mono: false },
    Format { id: 11, name: "rgba16", bytes: 8, mono: false },
    Format { id: 2, name: "rgba32f", bytes: 16, mono: false },
    Format { id: 61, name: "r8", bytes: 1, mono: true },
    Format { id: 56, name: "r16", bytes: 2, mono: true },
    Format { id: 54, name: "r16f", bytes: 2, mono: true },
    Format { id: 41, name: "r32f", bytes: 4, mono: true },
];

fn format(arg: &str) -> Res<&'static Format> {
    FORMATS
        .iter()
        .find(|f| f.name == arg || f.id.to_string() == arg)
        .ok_or_else(|| {
            let names: Vec<_> = FORMATS.iter().map(|f| f.name).collect();
            format!("unknown format '{arg}' (one of: {})", names.join(", ")).into()
        })
}

/// f32 → IEEE half, truncating; enough for values in 0..1.
fn half(v: f32) -> u16 {
    let bits = v.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exp = ((bits >> 23) & 0xff) as i32 - 127 + 15;
    let man = bits & 0x7f_ffff;
    if exp <= 0 {
        sign
    } else if exp >= 31 {
        sign | 0x7c00
    } else {
        sign | ((exp as u16) << 10) | ((man >> 13) as u16)
    }
}

fn unorm(v: f32, max: u32) -> u32 {
    (v.clamp(0.0, 1.0) * max as f32).round() as u32
}

/// One pixel of `px` in the format's memory layout.
fn encode(f: &Format, px: [f32; 4], out: &mut Vec<u8>) {
    let [r, g, b, a] = px;
    match f.id {
        87 | 91 => out.extend([unorm(b, 255), unorm(g, 255), unorm(r, 255), unorm(a, 255)].map(|c| c as u8)),
        88 => out.extend([unorm(b, 255), unorm(g, 255), unorm(r, 255), 255].map(|c| c as u8)),
        28 | 29 => out.extend([r, g, b, a].map(|c| unorm(c, 255) as u8)),
        24 => {
            let v = unorm(r, 1023) | unorm(g, 1023) << 10 | unorm(b, 1023) << 20 | unorm(a, 3) << 30;
            out.extend(v.to_le_bytes());
        }
        10 => [r, g, b, a].iter().for_each(|&c| out.extend(half(c).to_le_bytes())),
        11 => [r, g, b, a].iter().for_each(|&c| out.extend((unorm(c, 65535) as u16).to_le_bytes())),
        2 => [r, g, b, a].iter().for_each(|&c| out.extend(c.to_le_bytes())),
        61 => out.push(unorm(r, 255) as u8),
        56 => out.extend((unorm(r, 65535) as u16).to_le_bytes()),
        54 => out.extend(half(r).to_le_bytes()),
        41 => out.extend(r.to_le_bytes()),
        _ => unreachable!("format table and encoder out of sync"),
    }
}

/// What bar `i` should look like once converted to 8-bit RGB.
fn expected(f: &Format, i: usize) -> [f32; 3] {
    if f.mono {
        let v = MONO[i] * 255.0;
        [v, v, v]
    } else {
        let [r, g, b, _] = BARS[i];
        [r * 255.0, g * 255.0, b * 255.0]
    }
}

fn bar_pixel(f: &Format, i: usize) -> [f32; 4] {
    if f.mono {
        [MONO[i], 0.0, 0.0, 1.0]
    } else {
        BARS[i]
    }
}

pub fn run_pattern(a: &Args) -> Res<()> {
    let name = a.str("name", "kf-fmt-src");
    let f = format(&a.str("format", "bgra8"))?;
    let w = a.num("width", 1280u32)?;
    let h = a.num("height", 720u32)?;
    let fps = a.num("fps", 60.0f64)?;
    let duration_s = a.num("duration", 60.0f64)?;

    let dev = Device::new()?;
    let sender = Sender::with_format(&dev, &name, w, h, f.id)
        .map_err(|e| format!("cannot create a {} ({}) shared texture: {e}", f.name, f.id))?;

    let mut row = Vec::with_capacity(w as usize * f.bytes);
    for x in 0..w {
        encode(f, bar_pixel(f, (x * 4 / w) as usize), &mut row);
    }
    let pixels = row.repeat(h as usize);
    sender.fill(&dev, &pixels, row.len())?;
    println!("pattern: '{name}' {w}x{h} {} (DXGI {}) at {fps} fps", f.name, f.id);

    let pacer = Pacer::new()?;
    let period_us = (1e6 / fps) as i64;
    let end = now_us() + (duration_s * 1e6) as i64;
    let mut next = now_us();
    while now_us() < end {
        sender.tick();
        next += period_us;
        pacer.sleep_until(next);
    }
    Ok(())
}

pub fn run_check(a: &Args) -> Res<()> {
    let name = a.str("name", "bench-out");
    let f = format(&a.str("expect", "bgra8"))?;
    let wait_s = a.num("wait", 60.0f64)?;
    let settle_s = a.num("settle", 3.0f64)?;
    let tolerance = a.num("tolerance", 40.0f32)?;

    let dev = Device::new()?;
    let mut rx = Receiver::new(&name, false);
    let pacer = Pacer::new()?;
    let give_up = now_us() + (wait_s * 1e6) as i64;
    while !rx.refresh(&dev)? {
        if now_us() > give_up {
            return Err(format!("sender '{name}' never appeared").into());
        }
        pacer.sleep_until(now_us() + 200_000);
    }
    // The first frames of a stream are the encoder warming up: let it settle.
    pacer.sleep_until(now_us() + (settle_s * 1e6) as i64);
    rx.refresh(&dev)?;

    let src = rx.texture().ok_or("sender texture vanished")?.clone();
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    unsafe { src.GetDesc(&mut desc) };
    let fmt = desc.Format.0 as u32;
    let rgba_order = match fmt {
        87 | 88 | 90 | 91 => false,
        27 | 28 | 29 => true,
        other => return Err(format!("'{name}' publishes DXGI format {other}: not an 8-bit RGB one").into()),
    };
    let (w, h) = (desc.Width, desc.Height);
    let staging = dev.staging_texture_in(fmt, w, h)?;
    rx.locked(|| dev.copy(&staging, &src)).ok_or("sender mutex busy")??;

    let (mut ok, block) = (true, 8u32);
    let samples = dev.read(&staging, |bytes, pitch| {
        (0..4usize)
            .map(|i| {
                let cx = (2 * i as u32 + 1) * w / 8;
                let cy = h / 2;
                let mut sum = [0f32; 3];
                for y in cy - block..cy + block {
                    for x in cx - block..cx + block {
                        let p = &bytes[y as usize * pitch + x as usize * 4..][..4];
                        let (r, g, b) = if rgba_order { (p[0], p[1], p[2]) } else { (p[2], p[1], p[0]) };
                        sum[0] += r as f32;
                        sum[1] += g as f32;
                        sum[2] += b as f32;
                    }
                }
                let n = (4 * block * block) as f32;
                [sum[0] / n, sum[1] / n, sum[2] / n]
            })
            .collect::<Vec<_>>()
    })?;

    println!("check: '{name}' {w}x{h} DXGI {fmt}, expecting the {} bars", f.name);
    for (i, got) in samples.iter().enumerate() {
        let want = expected(f, i);
        let err = (0..3).map(|c| (got[c] - want[c]).abs()).fold(0f32, f32::max);
        let pass = err <= tolerance;
        ok &= pass;
        println!("  bar {i}: got ({:>3.0},{:>3.0},{:>3.0}) want ({:>3.0},{:>3.0},{:>3.0}) max err {:>5.1}  {}",
                 got[0], got[1], got[2], want[0], want[1], want[2], err, if pass { "ok" } else { "FAIL" });
    }
    if ok {
        println!("check: PASS");
        Ok(())
    } else {
        Err("colour bars do not match".into())
    }
}
